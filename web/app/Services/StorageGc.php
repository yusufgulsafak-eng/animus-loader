<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use App\Core\Env;
use Throwable;

/**
 * Dosya sistemi çöp toplayıcı.
 *
 * Kural: dosya silme işlemi ASLA açık bir transaction içinde yapılmaz.
 * DB commit edilir, ardından dosya karantinaya alınır. Karantina başarısız
 * olursa kayıt storage_gc_queue tablosuna düşer ve maintenance script'i
 * tekrar dener. Böylece "DB silindi ama disk doldu" durumu oluşmaz.
 */
final class StorageGc
{
    public const AREA_PATCH = 'patch';
    public const AREA_LOADER = 'loader';
    public const AREA_IMAGE = 'image';
    public const AREA_BRANDING = 'branding';

    public const AREAS = [self::AREA_PATCH, self::AREA_LOADER, self::AREA_IMAGE, self::AREA_BRANDING];

    /**
     * Dosyayı kalıcı silmek yerine storage/trash altına taşır.
     * Böylece yanlış silmede dosya TRASH_RETENTION_DAYS boyunca geri alınabilir.
     */
    public function quarantine(string $area, ?string $absolutePath, string $reason = 'delete'): bool
    {
        if (!in_array($area, self::AREAS, true) || $absolutePath === null || $absolutePath === '') {
            return false;
        }
        if (!is_file($absolutePath)) {
            return true; // Dosya zaten yok, iş tamam.
        }

        $name = basename($absolutePath);
        try {
            $directory = $this->trashDir() . DIRECTORY_SEPARATOR . $area . DIRECTORY_SEPARATOR . date('Y-m-d');
            if (!is_dir($directory) && !mkdir($directory, 0750, true) && !is_dir($directory)) {
                throw new \RuntimeException('Karantina dizini oluşturulamadı.');
            }
            $target = $directory . DIRECTORY_SEPARATOR . $name;
            if (is_file($target)) {
                $target = $directory . DIRECTORY_SEPARATOR . bin2hex(random_bytes(4)) . '-' . $name;
            }
            if (!@rename($absolutePath, $target)) {
                throw new \RuntimeException('Dosya karantinaya taşınamadı.');
            }
            return true;
        } catch (Throwable $error) {
            $this->queue($area, $name, $absolutePath, $reason, $error->getMessage());
            return false;
        }
    }

    public function queue(string $area, string $storageName, string $absolutePath, string $reason, ?string $error = null): void
    {
        try {
            Database::connection()->prepare(
                'INSERT INTO storage_gc_queue(area,storage_name,absolute_path,reason,attempts,last_error)
                 VALUES(?,?,?,?,1,?)
                 ON DUPLICATE KEY UPDATE attempts=attempts+1,last_error=VALUES(last_error),resolved_at=NULL'
            )->execute([$area, $storageName, $absolutePath, $reason, $error === null ? null : mb_substr($error, 0, 500)]);
        } catch (Throwable) {
            // Kuyruk yazılamıyorsa istek akışını bozma; orphan tarama yine yakalar.
        }
    }

    /**
     * Bekleyen kuyruğu işler. Cron / maintenance script tarafından çağrılır.
     *
     * @return array{processed:int,resolved:int,failed:int}
     */
    public function runQueue(int $limit = 200): array
    {
        $pdo = Database::connection();
        $stmt = $pdo->prepare('SELECT id,area,storage_name,absolute_path,reason FROM storage_gc_queue WHERE resolved_at IS NULL AND attempts < 25 ORDER BY id LIMIT ' . max(1, min(1000, $limit)));
        $stmt->execute();
        $rows = $stmt->fetchAll();

        $resolved = 0;
        $failed = 0;
        foreach ($rows as $row) {
            $done = !is_file($row['absolute_path']) || $this->quarantine((string) $row['area'], (string) $row['absolute_path'], (string) $row['reason']);
            if ($done && !is_file($row['absolute_path'])) {
                $pdo->prepare('UPDATE storage_gc_queue SET resolved_at=NOW(),last_error=NULL WHERE id=?')->execute([$row['id']]);
                $resolved++;
                continue;
            }
            $pdo->prepare('UPDATE storage_gc_queue SET attempts=attempts+1 WHERE id=?')->execute([$row['id']]);
            $failed++;
        }

        $pdo->exec('DELETE FROM storage_gc_queue WHERE resolved_at IS NOT NULL AND resolved_at < DATE_SUB(NOW(), INTERVAL 30 DAY)');
        return ['processed' => count($rows), 'resolved' => $resolved, 'failed' => $failed];
    }

    /**
     * Karantinadaki eski dosyaları kalıcı siler.
     *
     * @return array{deleted:int,bytes:int}
     */
    public function purgeTrash(?int $olderThanDays = null): array
    {
        $days = $olderThanDays ?? Env::int('TRASH_RETENTION_DAYS', 14);
        $threshold = time() - ($days * 86400);
        $root = $this->trashDir();
        $deleted = 0;
        $bytes = 0;

        if (!is_dir($root)) {
            return ['deleted' => 0, 'bytes' => 0];
        }
        foreach (self::AREAS as $area) {
            foreach (glob($root . DIRECTORY_SEPARATOR . $area . DIRECTORY_SEPARATOR . '*', GLOB_ONLYDIR) ?: [] as $dayDir) {
                $day = strtotime(basename($dayDir)) ?: 0;
                if ($day === 0 || $day > $threshold) {
                    continue;
                }
                foreach (glob($dayDir . DIRECTORY_SEPARATOR . '*') ?: [] as $file) {
                    if (!is_file($file)) {
                        continue;
                    }
                    $size = (int) @filesize($file);
                    if (@unlink($file)) {
                        $deleted++;
                        $bytes += $size;
                    }
                }
                @rmdir($dayDir);
            }
        }
        return ['deleted' => $deleted, 'bytes' => $bytes];
    }

    /**
     * DB'de karşılığı olmayan (orphan) dosyaları listeler.
     *
     * @return array<int,array{area:string,name:string,path:string,size:int,modified:string}>
     */
    public function scanOrphans(): array
    {
        $pdo = Database::connection();
        $referenced = [
            self::AREA_PATCH => array_flip(array_map('strval', $pdo->query("SELECT storage_name FROM patch_archives WHERE source_type='server'")->fetchAll(\PDO::FETCH_COLUMN) ?: [])),
            self::AREA_LOADER => array_flip(array_map('strval', $pdo->query('SELECT storage_name FROM loader_versions')->fetchAll(\PDO::FETCH_COLUMN) ?: [])),
            self::AREA_IMAGE => array_flip($this->referencedImageNames($pdo)),
            self::AREA_BRANDING => array_flip($this->referencedBrandingNames($pdo)),
        ];

        $orphans = [];
        foreach ($this->directories() as $area => $directory) {
            if (!is_dir($directory)) {
                continue;
            }
            foreach (glob($directory . DIRECTORY_SEPARATOR . '*') ?: [] as $file) {
                $name = basename($file);
                if (!is_file($file) || $name === '.gitkeep' || $name === '.htaccess') {
                    continue;
                }
                if (isset($referenced[$area][$name])) {
                    continue;
                }
                // Yükleme sırasında oluşmuş taze dosyaları (henüz commit edilmemiş) atla.
                if ((int) @filemtime($file) > time() - 3600) {
                    continue;
                }
                $orphans[] = [
                    'area' => $area,
                    'name' => $name,
                    'path' => $file,
                    'size' => (int) @filesize($file),
                    'modified' => date('Y-m-d H:i:s', (int) @filemtime($file)),
                ];
            }
        }
        return $orphans;
    }

    /**
     * Orphan dosyaları karantinaya alır.
     *
     * @return array{count:int,bytes:int}
     */
    public function quarantineOrphans(): array
    {
        $count = 0;
        $bytes = 0;
        foreach ($this->scanOrphans() as $orphan) {
            if ($this->quarantine($orphan['area'], $orphan['path'], 'orphan')) {
                $count++;
                $bytes += $orphan['size'];
            }
        }
        return ['count' => $count, 'bytes' => $bytes];
    }

    /** @return array{pending:int,failed:int,trash_files:int,trash_bytes:int} */
    public function status(): array
    {
        $pdo = Database::connection();
        $pending = (int) $pdo->query('SELECT COUNT(*) FROM storage_gc_queue WHERE resolved_at IS NULL')->fetchColumn();
        $failed = (int) $pdo->query('SELECT COUNT(*) FROM storage_gc_queue WHERE resolved_at IS NULL AND attempts >= 25')->fetchColumn();

        $files = 0;
        $bytes = 0;
        foreach (self::AREAS as $area) {
            foreach (glob($this->trashDir() . DIRECTORY_SEPARATOR . $area . DIRECTORY_SEPARATOR . '*' . DIRECTORY_SEPARATOR . '*') ?: [] as $file) {
                if (is_file($file)) {
                    $files++;
                    $bytes += (int) @filesize($file);
                }
            }
        }
        return ['pending' => $pending, 'failed' => $failed, 'trash_files' => $files, 'trash_bytes' => $bytes];
    }

    /** @return array<string,string> */
    private function directories(): array
    {
        return [
            self::AREA_PATCH => (new PatchStorage())->directory(),
            self::AREA_LOADER => (new LoaderStorage())->directory(),
            self::AREA_IMAGE => (new ImageStorage())->directory(),
            self::AREA_BRANDING => (new BrandingMediaStorage())->directory(),
        ];
    }

    /** @return array<int,string> */
    private function referencedImageNames(\PDO $pdo): array
    {
        $names = [];
        $columns = ['local_cover_path', 'local_banner_path', 'local_icon_path', 'cover_path', 'banner_path', 'icon_path'];
        foreach ($columns as $column) {
            foreach ($pdo->query("SELECT {$column} FROM games WHERE {$column} IS NOT NULL")->fetchAll(\PDO::FETCH_COLUMN) ?: [] as $value) {
                $names[] = basename((string) $value);
            }
        }
        foreach ($pdo->query('SELECT image_path FROM banners')->fetchAll(\PDO::FETCH_COLUMN) ?: [] as $value) {
            $names[] = basename((string) $value);
        }
        return array_values(array_unique($names));
    }

    /** @return array<int,string> */
    private function referencedBrandingNames(\PDO $pdo): array
    {
        $row = $pdo->query('SELECT * FROM loader_config WHERE id=1')->fetch() ?: [];
        $names = [];
        foreach ($row as $key => $value) {
            if (is_string($value) && str_starts_with($value, '/media/branding/')) {
                $names[] = basename($value);
            }
        }
        return array_values(array_unique($names));
    }

    private function trashDir(): string
    {
        $configured = Env::get('TRASH_STORAGE_PATH', 'storage/trash') ?? 'storage/trash';
        return preg_match('~^[a-zA-Z]:[\\\\/]~', $configured)
            ? $configured
            : WEB_ROOT . DIRECTORY_SEPARATOR . str_replace(['/', '\\'], DIRECTORY_SEPARATOR, $configured);
    }
}
