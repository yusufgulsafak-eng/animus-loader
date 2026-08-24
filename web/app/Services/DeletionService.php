<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use DomainException;
use PDO;
use Throwable;

/**
 * Silme işlemlerinin tek merkezi.
 *
 * Kurallar:
 *  1. Her silme önce etki raporu üretebilir (describe*), UI onayı gerçek veriye dayanır.
 *  2. Yayında olan / bağımlılığı olan kayıt force olmadan silinmez.
 *  3. DB işlemleri transaction içinde, dosya işlemleri commit sonrası yapılır.
 *  4. Silinen kaydın tam kopyası audit_logs.before_json içine yazılır (geri dönüş referansı).
 *  5. Dosyalar unlink edilmez, storage/trash altına karantinaya alınır.
 */
final class DeletionService
{
    private StorageGc $gc;

    public function __construct()
    {
        $this->gc = new StorageGc();
    }

    // ------------------------------------------------------------------
    // Etki raporu
    // ------------------------------------------------------------------

    /**
     * UI "Sil" butonuna basmadan önce ne kaybedileceğini gösterir.
     */
    public function describe(string $entity, int $id): array
    {
        return match ($entity) {
            'game' => $this->describeGame($id),
            'patch_version' => $this->describePatchVersion($id),
            'loader_version' => $this->describeLoaderVersion($id),
            'category' => $this->describeCategory($id),
            'user' => $this->describeUser($id),
            default => throw new DomainException('Bu varlık türü için silme raporu yok.'),
        };
    }

    public function describeGame(int $id): array
    {
        $pdo = Database::connection();
        $game = $this->row($pdo, 'SELECT id,name,slug,is_active FROM games WHERE id=?', [$id], 'Oyun bulunamadı.');

        $stmt = $pdo->prepare(
            "SELECT COUNT(*) total,
                    SUM(pv.status='PUBLISHED') published,
                    COALESCE(SUM(pa.size_bytes),0) bytes
             FROM patch_versions pv
             JOIN patches p ON p.id=pv.patch_id
             LEFT JOIN patch_archives pa ON pa.patch_version_id=pv.id
             WHERE p.game_id=?"
        );
        $stmt->execute([$id]);
        $versions = $stmt->fetch() ?: ['total' => 0, 'published' => 0, 'bytes' => 0];

        $stmt = $pdo->prepare(
            'SELECT COUNT(*) FROM download_logs dl
             JOIN patch_archives pa ON pa.id=dl.patch_archive_id
             JOIN patch_versions pv ON pv.id=pa.patch_version_id
             JOIN patches p ON p.id=pv.patch_id WHERE p.game_id=?'
        );
        $stmt->execute([$id]);

        return [
            'entity' => 'game',
            'id' => $id,
            'label' => $game['name'],
            'blocking' => (int) $versions['published'] > 0
                ? ['Bu oyunun ' . (int) $versions['published'] . ' adet YAYINDA sürümü var. Silmek loader istemcilerinde yamayı anında kaldırır.']
                : [],
            'cascade' => [
                'patch_versions' => (int) $versions['total'],
                'published_versions' => (int) $versions['published'],
                'archive_bytes' => (int) $versions['bytes'],
                'download_logs_detached' => (int) $stmt->fetchColumn(),
            ],
            'requires_force' => (int) $versions['published'] > 0,
        ];
    }

    public function describePatchVersion(int $id): array
    {
        $pdo = Database::connection();
        $version = $this->row(
            $pdo,
            'SELECT pv.id,pv.version,pv.status,pv.channel,pv.patch_id,g.name game_name
             FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id WHERE pv.id=?',
            [$id],
            'Patch sürümü bulunamadı.'
        );

        $stmt = $pdo->prepare('SELECT COUNT(*) FROM patch_release_channels WHERE active_patch_version_id=?');
        $stmt->execute([$id]);
        $isActive = (int) $stmt->fetchColumn() > 0;

        $stmt = $pdo->prepare('SELECT source_type,size_bytes FROM patch_archives WHERE patch_version_id=?');
        $stmt->execute([$id]);
        $archive = $stmt->fetch() ?: null;

        $replacement = $this->findReplacement($pdo, (int) $version['patch_id'], (string) $version['channel'], $id);

        $blocking = [];
        if ($isActive) {
            $blocking[] = $replacement !== null
                ? 'Bu sürüm ' . $version['channel'] . ' kanalının aktif yayını. Silinirse kanal otomatik olarak ' . $replacement['version'] . ' sürümüne döner.'
                : 'Bu sürüm ' . $version['channel'] . ' kanalının aktif yayını ve yerine geçecek başka yayınlanmış sürüm yok. Silinirse bu kanalda yama kalmaz.';
        }

        return [
            'entity' => 'patch_version',
            'id' => $id,
            'label' => $version['game_name'] . ' ' . $version['version'] . ' (' . $version['channel'] . ')',
            'blocking' => $blocking,
            'cascade' => [
                'is_active_release' => $isActive,
                'replacement_version' => $replacement['version'] ?? null,
                'archive_bytes' => (int) ($archive['size_bytes'] ?? 0),
                'archive_source' => $archive['source_type'] ?? null,
            ],
            'requires_force' => $isActive,
        ];
    }

    public function describeLoaderVersion(int $id): array
    {
        $pdo = Database::connection();
        $version = $this->row($pdo, 'SELECT * FROM loader_versions WHERE id=?', [$id], 'Loader sürümü bulunamadı.');

        $stmt = $pdo->prepare('SELECT id FROM loader_versions WHERE channel=? ORDER BY published_at DESC,id DESC LIMIT 1');
        $stmt->execute([$version['channel']]);
        $isLatest = (int) $stmt->fetchColumn() === $id;

        return [
            'entity' => 'loader_version',
            'id' => $id,
            'label' => $version['version'] . ' (' . $version['channel'] . ')',
            'blocking' => $isLatest ? ['Bu, ' . $version['channel'] . ' kanalının en güncel loader paketi. Silinirse istemciler bir önceki sürüme düşer.'] : [],
            'cascade' => ['package_bytes' => (int) $version['size_bytes']],
            'requires_force' => $isLatest,
        ];
    }

    public function describeCategory(int $id): array
    {
        $pdo = Database::connection();
        $category = $this->row($pdo, 'SELECT * FROM categories WHERE id=?', [$id], 'Kategori bulunamadı.');
        $stmt = $pdo->prepare('SELECT COUNT(*) FROM game_categories WHERE category_id=?');
        $stmt->execute([$id]);
        $games = (int) $stmt->fetchColumn();

        return [
            'entity' => 'category',
            'id' => $id,
            'label' => $category['name'],
            'blocking' => $games > 0 ? [$games . ' oyun bu kategoriye bağlı. Silinirse oyunlar kategorisiz kalır (oyunlar silinmez).'] : [],
            'cascade' => ['games_unlinked' => $games],
            'requires_force' => $games > 0,
        ];
    }

    public function describeUser(int $id): array
    {
        $pdo = Database::connection();
        $user = $this->row($pdo, 'SELECT id,email,display_name,role FROM users WHERE id=?', [$id], 'Kullanıcı bulunamadı.');
        $stmt = $pdo->prepare('SELECT COUNT(*) FROM subscriptions WHERE user_id=?');
        $stmt->execute([$id]);

        return [
            'entity' => 'user',
            'id' => $id,
            'label' => $user['email'],
            'blocking' => $user['role'] === 'super_admin' ? ['Süper admin hesabı siliniyor.'] : [],
            'cascade' => [
                'subscriptions' => (int) $stmt->fetchColumn(),
                'note' => 'Oturum tokenları ve abonelikler silinir; indirme geçmişi anonimleştirilerek korunur.',
            ],
            'requires_force' => $user['role'] === 'super_admin',
        ];
    }

    // ------------------------------------------------------------------
    // Silme işlemleri
    // ------------------------------------------------------------------

    public function deleteGame(int $id, int $actor, bool $force = false): array
    {
        $pdo = Database::connection();
        $report = $this->describeGame($id);
        if ($report['requires_force'] && !$force) {
            throw new DomainException(implode(' ', $report['blocking']) . ' Kalıcı silmek için onay kutusunu işaretleyin.');
        }

        $files = [];
        $pdo->beginTransaction();
        try {
            $game = $this->row($pdo, 'SELECT * FROM games WHERE id=? FOR UPDATE', [$id], 'Oyun bulunamadı.');

            $stmt = $pdo->prepare(
                "SELECT pa.storage_name FROM patch_archives pa
                 JOIN patch_versions pv ON pv.id=pa.patch_version_id
                 JOIN patches p ON p.id=pv.patch_id
                 WHERE p.game_id=? AND pa.source_type='server'"
            );
            $stmt->execute([$id]);
            $patchStorage = new PatchStorage();
            foreach ($stmt->fetchAll(PDO::FETCH_COLUMN) ?: [] as $storageName) {
                $files[] = [StorageGc::AREA_PATCH, $patchStorage->path((string) $storageName)];
            }

            $images = new ImageStorage();
            foreach (['local_cover_path', 'local_banner_path', 'local_icon_path'] as $column) {
                if ($path = $images->absolutePath($game[$column] ?? null)) {
                    $files[] = [StorageGc::AREA_IMAGE, $path];
                }
            }

            // patch_release_channels FK'si RESTRICT: önce kanal işaretçileri kaldırılmalı.
            $pdo->prepare('DELETE prc FROM patch_release_channels prc JOIN patches p ON p.id=prc.patch_id WHERE p.game_id=?')->execute([$id]);
            $pdo->prepare('DELETE FROM games WHERE id=?')->execute([$id]);

            (new AuditService())->write($actor, 'game.deleted', 'game', $id, $game, ['cascade' => $report['cascade'], 'forced' => $force]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            throw $error;
        }

        $this->quarantineAll($files);
        return ['deleted' => 'game', 'id' => $id, 'label' => $report['label'], 'cascade' => $report['cascade'], 'files_quarantined' => count($files)];
    }

    public function deletePatchVersion(int $id, int $actor, bool $force = false): array
    {
        $pdo = Database::connection();
        $report = $this->describePatchVersion($id);
        if ($report['requires_force'] && !$force) {
            throw new DomainException(implode(' ', $report['blocking']) . ' Kalıcı silmek için onay kutusunu işaretleyin.');
        }

        $files = [];
        $replacement = null;
        $pdo->beginTransaction();
        try {
            $version = $this->row(
                $pdo,
                'SELECT pv.*,p.game_id FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id WHERE pv.id=? FOR UPDATE',
                [$id],
                'Patch sürümü bulunamadı.'
            );

            $stmt = $pdo->prepare('SELECT id,source_type,storage_name,size_bytes FROM patch_archives WHERE patch_version_id=?');
            $stmt->execute([$id]);
            if ($archive = $stmt->fetch()) {
                if (($archive['source_type'] ?? 'server') === 'server') {
                    $files[] = [StorageGc::AREA_PATCH, (new PatchStorage())->path((string) $archive['storage_name'])];
                }
            }

            // Aktif yayınsa: kanalı bir önceki yayınlanmış sürüme devret, yoksa kanal kaydını kaldır.
            $stmt = $pdo->prepare('SELECT id FROM patch_release_channels WHERE active_patch_version_id=? FOR UPDATE');
            $stmt->execute([$id]);
            if ($stmt->fetchColumn()) {
                $replacement = $this->findReplacement($pdo, (int) $version['patch_id'], (string) $version['channel'], $id);
                if ($replacement !== null) {
                    $pdo->prepare('UPDATE patch_release_channels SET active_patch_version_id=?,updated_by=? WHERE patch_id=? AND channel=?')
                        ->execute([$replacement['id'], $actor, $version['patch_id'], $version['channel']]);
                    $pdo->prepare("UPDATE patch_versions SET status='PUBLISHED' WHERE id=? AND status<>'PUBLISHED'")->execute([$replacement['id']]);
                } else {
                    $pdo->prepare('DELETE FROM patch_release_channels WHERE patch_id=? AND channel=?')->execute([$version['patch_id'], $version['channel']]);
                }
            }

            // patch_archives, patch_install_actions, download_tokens CASCADE ile gider.
            // download_logs.patch_archive_id SET NULL: indirme istatistiği korunur.
            $pdo->prepare('DELETE FROM patch_versions WHERE id=?')->execute([$id]);

            (new AuditService())->write($actor, 'patch_version.deleted', 'patch_version', $id, $version, [
                'replacement_version_id' => $replacement['id'] ?? null,
                'forced' => $force,
            ]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            throw $error;
        }

        $this->quarantineAll($files);
        return [
            'deleted' => 'patch_version',
            'id' => $id,
            'label' => $report['label'],
            'replacement_version' => $replacement['version'] ?? null,
            'files_quarantined' => count($files),
        ];
    }

    public function deleteLoaderVersion(int $id, int $actor, bool $force = false): array
    {
        $pdo = Database::connection();
        $report = $this->describeLoaderVersion($id);
        if ($report['requires_force'] && !$force) {
            throw new DomainException(implode(' ', $report['blocking']) . ' Kalıcı silmek için onay kutusunu işaretleyin.');
        }

        $files = [];
        $pdo->beginTransaction();
        try {
            $version = $this->row($pdo, 'SELECT * FROM loader_versions WHERE id=? FOR UPDATE', [$id], 'Loader sürümü bulunamadı.');
            $files[] = [StorageGc::AREA_LOADER, (new LoaderStorage())->path((string) $version['storage_name'])];
            $pdo->prepare('DELETE FROM loader_versions WHERE id=?')->execute([$id]);
            (new AuditService())->write($actor, 'loader_version.deleted', 'loader_version', $id, $version, ['forced' => $force]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            throw $error;
        }

        $this->quarantineAll($files);
        return ['deleted' => 'loader_version', 'id' => $id, 'label' => $report['label'], 'files_quarantined' => count($files)];
    }

    public function deleteCategory(int $id, int $actor, bool $force = false): array
    {
        $pdo = Database::connection();
        $report = $this->describeCategory($id);
        if ($report['requires_force'] && !$force) {
            throw new DomainException(implode(' ', $report['blocking']) . ' Devam etmek için onay kutusunu işaretleyin.');
        }

        $pdo->beginTransaction();
        try {
            $category = $this->row($pdo, 'SELECT * FROM categories WHERE id=? FOR UPDATE', [$id], 'Kategori bulunamadı.');
            $pdo->prepare('DELETE FROM categories WHERE id=?')->execute([$id]);
            (new AuditService())->write($actor, 'category.deleted', 'category', $id, $category, ['games_unlinked' => $report['cascade']['games_unlinked'], 'forced' => $force]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            throw $error;
        }
        return ['deleted' => 'category', 'id' => $id, 'label' => $report['label'], 'cascade' => $report['cascade']];
    }

    public function deleteAnnouncement(int $id, int $actor): array
    {
        $pdo = Database::connection();
        $row = $this->row($pdo, 'SELECT * FROM announcements WHERE id=?', [$id], 'Duyuru bulunamadı.');
        $pdo->prepare('DELETE FROM announcements WHERE id=?')->execute([$id]);
        (new AuditService())->write($actor, 'announcement.deleted', 'announcement', $id, $row, null);
        return ['deleted' => 'announcement', 'id' => $id, 'label' => $row['title']];
    }

    public function deleteBanner(int $id, int $actor): array
    {
        $pdo = Database::connection();
        $row = $this->row($pdo, 'SELECT * FROM banners WHERE id=?', [$id], 'Banner bulunamadı.');
        $path = (new ImageStorage())->absolutePath($row['image_path'] ?? null);
        $pdo->prepare('DELETE FROM banners WHERE id=?')->execute([$id]);
        (new AuditService())->write($actor, 'banner.deleted', 'banner', $id, $row, null);
        if ($path !== null) {
            $this->quarantineAll([[StorageGc::AREA_IMAGE, $path]]);
        }
        return ['deleted' => 'banner', 'id' => $id, 'label' => $row['title']];
    }

    public function deleteSubscription(int $id, int $actor): array
    {
        $pdo = Database::connection();
        $row = $this->row($pdo, 'SELECT * FROM subscriptions WHERE id=?', [$id], 'Abonelik bulunamadı.');
        $pdo->prepare('DELETE FROM subscriptions WHERE id=?')->execute([$id]);
        (new AuditService())->write($actor, 'subscription.deleted', 'subscription', $id, $row, null);
        return ['deleted' => 'subscription', 'id' => $id, 'label' => $row['plan_name']];
    }

    public function deleteUser(int $id, int $actor, string $actorRole, bool $force = false): array
    {
        if ($id === $actor) {
            throw new DomainException('Kendi hesabınızı silemezsiniz.');
        }

        $pdo = Database::connection();
        $pdo->beginTransaction();
        try {
            $user = $this->row($pdo, 'SELECT * FROM users WHERE id=? FOR UPDATE', [$id], 'Kullanıcı bulunamadı.');

            if ($user['role'] === 'super_admin') {
                if ($actorRole !== 'super_admin') {
                    throw new DomainException('Süper admin hesabını yalnız başka bir süper admin silebilir.');
                }
                if (!$force) {
                    throw new DomainException('Süper admin siliniyor. Devam etmek için onay kutusunu işaretleyin.');
                }
            }
            $remaining = (int) $pdo->query("SELECT COUNT(*) FROM users WHERE status='active' AND role IN ('admin','super_admin')")->fetchColumn();
            if (in_array($user['role'], ['admin', 'super_admin'], true) && $user['status'] === 'active' && $remaining <= 1) {
                throw new DomainException('Sistemde en az bir aktif admin kalmalı.');
            }

            unset($user['password_hash']);
            // api_tokens, download_tokens, subscriptions, password_reset_tokens CASCADE ile gider.
            // download_logs / audit_logs SET NULL: kayıtlar anonimleşir ama kaybolmaz.
            $pdo->prepare('DELETE FROM users WHERE id=?')->execute([$id]);
            (new AuditService())->write($actor, 'user.deleted', 'user', $id, $user, ['forced' => $force]);
            $pdo->commit();
            return ['deleted' => 'user', 'id' => $id, 'label' => $user['email']];
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) {
                $pdo->rollBack();
            }
            throw $error;
        }
    }

    // ------------------------------------------------------------------
    // Bakım
    // ------------------------------------------------------------------

    /** Süresi dolmuş tokenları ve rate limit kayıtlarını temizler. */
    public function purgeExpiredTokens(): array
    {
        $pdo = Database::connection();
        return [
            'download_tokens' => $pdo->exec('DELETE FROM download_tokens WHERE expires_at < NOW() OR used_at IS NOT NULL'),
            'api_tokens' => $pdo->exec('DELETE FROM api_tokens WHERE expires_at < NOW()'),
            'password_reset_tokens' => $pdo->exec('DELETE FROM password_reset_tokens WHERE expires_at < NOW() OR used_at IS NOT NULL'),
            'rate_limits' => $pdo->exec('DELETE FROM rate_limits WHERE expires_at < NOW()'),
        ];
    }

    /** Belirtilen günden eski indirme loglarını siler. */
    public function pruneDownloadLogs(int $days, int $actor): array
    {
        $days = max(1, min(3650, $days));
        $stmt = Database::connection()->prepare('DELETE FROM download_logs WHERE created_at < DATE_SUB(NOW(), INTERVAL ? DAY)');
        $stmt->execute([$days]);
        $deleted = $stmt->rowCount();
        (new AuditService())->write($actor, 'download_logs.pruned', 'download_log', null, null, ['days' => $days, 'deleted' => $deleted]);
        return ['deleted' => $deleted, 'days' => $days];
    }

    /**
     * Yayınlanmamış, eski ve terk edilmiş draft sürümleri toplu siler.
     */
    public function pruneStaleDrafts(int $days, int $actor, bool $apply = false): array
    {
        $days = max(1, min(3650, $days));
        $stmt = Database::connection()->prepare(
            "SELECT pv.id,pv.version,g.name game_name FROM patch_versions pv
             JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id
             WHERE pv.status='DRAFT' AND pv.published_at IS NULL
               AND pv.created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
               AND NOT EXISTS (SELECT 1 FROM patch_release_channels prc WHERE prc.active_patch_version_id=pv.id)
             ORDER BY pv.created_at LIMIT 500"
        );
        $stmt->execute([$days]);
        $candidates = $stmt->fetchAll();

        if (!$apply) {
            return ['mode' => 'preview', 'days' => $days, 'candidates' => $candidates, 'count' => count($candidates)];
        }
        $deleted = 0;
        foreach ($candidates as $candidate) {
            try {
                $this->deletePatchVersion((int) $candidate['id'], $actor, false);
                $deleted++;
            } catch (Throwable) {
                // Tek kayıt hatası toplu işlemi durdurmaz.
            }
        }
        return ['mode' => 'applied', 'days' => $days, 'deleted' => $deleted, 'count' => count($candidates)];
    }

    // ------------------------------------------------------------------

    /** @param array<int,array{0:string,1:string}> $files */
    private function quarantineAll(array $files): void
    {
        foreach ($files as [$area, $path]) {
            $this->gc->quarantine($area, $path);
        }
    }

    private function findReplacement(PDO $pdo, int $patchId, string $channel, int $excludeId): ?array
    {
        $stmt = $pdo->prepare(
            "SELECT id,version FROM patch_versions
             WHERE patch_id=? AND channel=? AND id<>? AND status IN ('PUBLISHED','ARCHIVED')
             ORDER BY FIELD(status,'PUBLISHED','ARCHIVED'), published_at DESC, id DESC LIMIT 1"
        );
        $stmt->execute([$patchId, $channel, $excludeId]);
        return $stmt->fetch() ?: null;
    }

    private function row(PDO $pdo, string $sql, array $params, string $notFound): array
    {
        $stmt = $pdo->prepare($sql);
        $stmt->execute($params);
        $row = $stmt->fetch();
        if (!$row) {
            throw new DomainException($notFound);
        }
        return $row;
    }
}
