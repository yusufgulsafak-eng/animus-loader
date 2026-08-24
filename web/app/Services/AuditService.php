<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;

final class AuditService
{
    public function write(?int $actor, string $action, string $entity, string|int|null $id, ?array $before, ?array $after): void
    {
        $ipHash = hash('sha256', ($_SERVER['REMOTE_ADDR'] ?? 'cli') . '|' . ($_ENV['APP_KEY'] ?? 'local'));
        Database::connection()->prepare('INSERT INTO audit_logs(actor_user_id,action,entity_type,entity_id,before_json,after_json,ip_hash) VALUES(?,?,?,?,?,?,?)')->execute([
            $actor, $action, $entity, $id === null ? null : (string) $id,
            $before ? json_encode($before, JSON_UNESCAPED_UNICODE) : null,
            $after ? json_encode($after, JSON_UNESCAPED_UNICODE) : null, $ipHash,
        ]);
    }
}

