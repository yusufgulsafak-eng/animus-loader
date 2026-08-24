<?php
declare(strict_types=1);

namespace App\Support;

use App\Services\AdminService;
use App\Services\DeletionService;
use App\Services\ManifestService;
use App\Services\ManifestValidator;
use App\Services\StorageGc;
use DomainException;

/**
 * Tek admin action kayıt defteri.
 *
 * Önceden AdminController (web panel) ve ApiController (loader istemcisi) iki ayrı
 * match bloğu tutuyordu ve listeler birbirinden kaymıştı: delete_game yalnız API'de,
 * update_user yalnız web panelinde çalışıyordu. Artık iki controller da buraya
 * yönlendiriyor, yani her yeni işlem otomatik olarak her iki arayüzde de var.
 */
final class AdminActions
{
    /** Sadece super_admin çalıştırabilir. */
    private const SUPER_ADMIN_ONLY = ['delete_user'];

    /** @param array{id:int|string,role?:string} $actor */
    public static function dispatch(string $action, array $body, array $files, array $actor): mixed
    {
        $uid = (int) ($actor['id'] ?? 0);
        $role = (string) ($actor['role'] ?? 'admin');

        if ($action === '') {
            throw new DomainException('Action belirtilmedi.');
        }
        if (in_array($action, self::SUPER_ADMIN_ONLY, true) && $role !== 'super_admin') {
            throw new DomainException('Bu işlem için süper admin yetkisi gerekli.');
        }

        $admin = new AdminService();
        $trash = new DeletionService();
        $gc = new StorageGc();

        $id = static fn(string $key): int => (int) ($body[$key] ?? 0);
        $str = static fn(string $key, string $default = ''): string => (string) ($body[$key] ?? $default);
        $force = static fn(): bool => self::flag($body['force'] ?? null);

        return match ($action) {
            // ---------------- Panel ----------------
            'panel_data' => $admin->panelData(),
            'dashboard' => $admin->dashboard(),

            // ---------------- Oyunlar ----------------
            'save_game' => ['id' => $admin->saveGame($body['game'] ?? [], $uid)],
            'duplicate_game' => ['id' => $admin->duplicateGame($id('game_id'), $uid)],
            'set_game_status' => self::void(fn() => $admin->setGameStatus($id('game_id'), self::flag($body['active'] ?? false), $uid)),
            'delete_game' => $trash->deleteGame($id('game_id') ?: $id('id'), $uid, $force()),
            'upload_game_image' => ['path' => $admin->saveGameImage($id('game_id'), $str('kind'), $files['image'] ?? [], $uid)],
            'delete_game_image' => self::void(fn() => $admin->deleteGameImage($id('game_id'), $str('kind'), $uid)),

            // ---------------- Yamalar ----------------
            'inspect_external_patch' => $admin->inspectExternalPatch($str('url')),
            'create_patch' => ['id' => $admin->createPatchVersion($body, $files['archive'] ?? [], $uid)],
            'load_patch_builder' => $admin->builderData($id('version_id')),
            'save_actions' => self::void(fn() => $admin->saveActions($id('version_id'), $body['actions'] ?? [], $uid)),
            'test_manifest' => self::testManifest($id('version_id')),
            'publish_patch' => self::void(fn() => $admin->publish($id('version_id'), $uid)),
            'rollback_patch' => self::void(fn() => $admin->rollbackPatch($id('version_id'), $uid)),
            'set_patch_status' => self::void(fn() => $admin->setPatchStatus($id('version_id'), $str('status'), $uid)),
            'delete_patch_version' => $trash->deletePatchVersion($id('version_id') ?: $id('id'), $uid, $force()),

            // ---------------- Kategoriler ----------------
            'save_category' => ['id' => $admin->saveCategory($body['category'] ?? [], $uid)],
            'delete_category' => $trash->deleteCategory($id('id') ?: $id('category_id'), $uid, $force()),

            // ---------------- Duyurular / Bannerlar ----------------
            'save_announcement' => ['id' => $admin->saveAnnouncement($body['announcement'] ?? [], $uid)],
            'delete_announcement' => $trash->deleteAnnouncement($id('id'), $uid),
            'save_banner' => ['id' => $admin->saveBanner($body, $files['image'] ?? [], $uid)],
            'delete_banner' => $trash->deleteBanner($id('id'), $uid),

            // ---------------- Kullanıcılar / Abonelikler ----------------
            'update_user' => self::void(fn() => $admin->updateUser($body['user'] ?? [], $uid)),
            'delete_user' => $trash->deleteUser($id('id') ?: $id('user_id'), $uid, $role, $force()),
            'save_subscription' => ['id' => $admin->saveSubscription($body['subscription'] ?? [], $uid)],
            'set_subscription_status' => self::void(fn() => $admin->setSubscriptionStatus($id('id'), $str('status'), $uid)),
            'delete_subscription' => $trash->deleteSubscription($id('id'), $uid),

            // ---------------- Loader ----------------
            'create_loader_version' => ['id' => $admin->createLoaderVersion($body, $files['package'] ?? [], $uid)],
            'delete_loader_version' => $trash->deleteLoaderVersion($id('id') ?: $id('version_id'), $uid, $force()),
            'save_loader_config' => self::void(fn() => $admin->saveLoaderConfig($body['config'] ?? [], $uid)),
            'save_branding_media' => $admin->saveBrandingMedia($body, $files, $uid),
            'reset_branding_media' => $admin->resetBrandingMedia($str('slot'), $uid),

            // ---------------- Silme öncesi etki raporu ----------------
            'describe_deletion' => $trash->describe($str('entity'), $id('id')),

            // ---------------- Bakım ----------------
            'storage_status' => $gc->status(),
            'run_storage_gc' => $gc->runQueue(),
            'scan_orphans' => ['orphans' => $gc->scanOrphans()],
            'purge_orphans' => $gc->quarantineOrphans(),
            'purge_trash' => $gc->purgeTrash(isset($body['days']) ? $id('days') : null),
            'purge_expired_tokens' => $trash->purgeExpiredTokens(),
            'prune_download_logs' => $trash->pruneDownloadLogs($id('days') ?: 90, $uid),
            'prune_stale_drafts' => $trash->pruneStaleDrafts($id('days') ?: 30, $uid, self::flag($body['apply'] ?? false)),

            default => throw new DomainException('Bilinmeyen admin işlemi: ' . $action),
        };
    }

    /** Checkbox / JSON / form-data farklarını tek yerde normalize eder. */
    public static function flag(mixed $value): bool
    {
        if (is_bool($value)) {
            return $value;
        }
        if (is_int($value)) {
            return $value === 1;
        }
        if (is_string($value)) {
            return in_array(strtolower(trim($value)), ['1', 'true', 'on', 'yes', 'evet'], true);
        }
        return false;
    }

    private static function testManifest(int $versionId): array
    {
        $manifest = (new ManifestService())->build($versionId);
        return ['manifest' => $manifest, 'errors' => (new ManifestValidator())->validate($manifest)];
    }

    private static function void(callable $callback): null
    {
        $callback();
        return null;
    }
}
