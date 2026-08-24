<?php
declare(strict_types=1);

/**
 * Periyodik bakım. cPanel cron örneği (her gece 04:00):
 *   /usr/local/bin/php /home/KULLANICI/api/web/scripts/maintenance.php
 *
 * Yaptıkları:
 *  - Süresi dolmuş indirme / API / şifre sıfırlama tokenlarını siler.
 *  - Silme sırasında diskten kaldırılamamış dosyaları (storage_gc_queue) tekrar dener.
 *  - TRASH_RETENTION_DAYS süresini dolduran karantina dosyalarını kalıcı siler.
 *  - DB'de karşılığı olmayan orphan dosyaları raporlar (--purge-orphans ile karantinaya alır).
 */

require __DIR__ . '/../app/bootstrap.php';

use App\Services\DeletionService;
use App\Services\StorageGc;

if (PHP_SAPI !== 'cli') {
    fwrite(STDERR, "Bu araç yalnız CLI üzerinden çalıştırılır.\n");
    exit(1);
}

$options = array_slice($argv, 1);
$purgeOrphans = in_array('--purge-orphans', $options, true);
$dryRun = in_array('--dry-run', $options, true);

$gc = new StorageGc();
$deletion = new DeletionService();

$line = static function (string $label, array $values): void {
    $parts = [];
    foreach ($values as $key => $value) {
        $parts[] = $key . '=' . (is_scalar($value) ? (string) $value : json_encode($value));
    }
    echo str_pad($label, 26) . implode('  ', $parts) . PHP_EOL;
};

echo '=== Animus bakım · ' . date('Y-m-d H:i:s') . ' ===' . PHP_EOL;

if ($dryRun) {
    echo 'DRY RUN: hiçbir şey silinmeyecek.' . PHP_EOL;
    $line('Storage durumu', $gc->status());
    $orphans = $gc->scanOrphans();
    $line('Orphan dosya', ['count' => count($orphans), 'bytes' => array_sum(array_column($orphans, 'size'))]);
    foreach (array_slice($orphans, 0, 25) as $orphan) {
        echo '  - [' . $orphan['area'] . '] ' . $orphan['name'] . ' (' . $orphan['size'] . ' bayt, ' . $orphan['modified'] . ')' . PHP_EOL;
    }
    exit(0);
}

$line('Token temizliği', $deletion->purgeExpiredTokens());
$line('GC kuyruğu', $gc->runQueue());
$line('Karantina temizliği', $gc->purgeTrash());

$orphans = $gc->scanOrphans();
$line('Orphan tarama', ['count' => count($orphans), 'bytes' => array_sum(array_column($orphans, 'size'))]);

if ($purgeOrphans && $orphans !== []) {
    $line('Orphan karantina', $gc->quarantineOrphans());
} elseif ($orphans !== []) {
    echo 'Orphan dosyaları karantinaya almak için: php scripts/maintenance.php --purge-orphans' . PHP_EOL;
}

$line('Storage durumu', $gc->status());
echo 'Bakım tamamlandı.' . PHP_EOL;
