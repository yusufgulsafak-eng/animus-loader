<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\PathGuard;

final class ManifestValidator
{
    public const ACTIONS = ['COPY_FILE','COPY_DIRECTORY','REPLACE_FILE','DELETE_FILE','DELETE_DIRECTORY','CREATE_DIRECTORY','MOVE_FILE','RENAME_FILE'];
    private const SOURCE_REQUIRED = ['COPY_FILE','COPY_DIRECTORY','REPLACE_FILE'];

    public function validate(array $manifest): array
    {
        $errors = [];
        if (($manifest['schema_version'] ?? null) !== 1) $errors[] = 'Desteklenmeyen schema_version.';
        foreach (['game','detection','patch','archive','install_actions','integrity','backup'] as $key) {
            if (!array_key_exists($key, $manifest)) $errors[] = "Eksik bölüm: {$key}";
        }
        if (empty($manifest['game']['id']) || empty($manifest['game']['slug'])) $errors[] = 'Oyun kimliği ve slug zorunludur.';
        if (!PathGuard::isSafeRelative((string)($manifest['detection']['executable'] ?? ''))) $errors[] = 'Yayın için güvenli bir ana executable tanımlanmalıdır.';
        if (empty($manifest['patch']['version'])) $errors[] = 'Patch version zorunludur.';
        if (!preg_match('/^[a-f0-9]{64}$/i', (string)($manifest['archive']['sha256'] ?? ''))) $errors[] = 'Archive SHA-256 geçersiz.';
        if ((int)($manifest['archive']['size'] ?? 0) < 1) $errors[] = 'Archive boyutu geçersiz.';
        if (!is_array($manifest['install_actions'] ?? null) || count($manifest['install_actions']) < 1) $errors[] = 'En az bir install action zorunludur.';

        foreach (($manifest['install_actions'] ?? []) as $index => $action) {
            $label = 'Action #' . ($index + 1);
            $type = $action['type'] ?? '';
            if (!in_array($type, self::ACTIONS, true)) $errors[] = "{$label}: action tipi izinli değil.";
            if (!isset($action['id']) || !preg_match('/^[a-f0-9-]{36}$/i', (string)$action['id'])) $errors[] = "{$label}: UUID geçersiz.";
            if (!PathGuard::isSafeRelative((string)($action['destination'] ?? ''))) $errors[] = "{$label}: hedef yol güvenli relative path değil.";
            if (in_array($type, self::SOURCE_REQUIRED, true) && !PathGuard::isSafeRelative((string)($action['source'] ?? ''))) $errors[] = "{$label}: kaynak yol güvenli değil.";
            if (in_array($type, ['MOVE_FILE','RENAME_FILE'], true) && !PathGuard::isSafeRelative((string)($action['source'] ?? ''))) $errors[] = "{$label}: oyun kökü kaynak yolu güvenli değil.";
        }
        foreach (($manifest['detection']['required_files'] ?? []) as $file) {
            if (!PathGuard::isSafeRelative((string)$file)) $errors[] = 'Detection required_file yolu güvenli değil.';
        }
        return array_values(array_unique($errors));
    }
}
