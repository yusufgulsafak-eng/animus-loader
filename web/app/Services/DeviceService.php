<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;

final class DeviceService
{
    private function validateUuid(string $deviceUuid): string
    {
        $deviceUuid = trim($deviceUuid);
        if (!preg_match('/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/i', $deviceUuid)) {
            throw new \DomainException('Cihaz kimliği geçersiz.');
        }
        return strtolower($deviceUuid);
    }

    public function activateForUser(int $userId, string $deviceUuid, string $deviceName): array
    {
        $deviceUuid = $this->validateUuid($deviceUuid);
        $deviceName = trim($deviceName);
        if ($deviceName === '') $deviceName = 'Windows PC';
        $deviceName = mb_substr($deviceName, 0, 190);

        $pdo = Database::connection();
        $pdo->beginTransaction();
        try {
            $stmt = $pdo->prepare('SELECT * FROM user_devices WHERE user_id=? AND device_uuid=? LIMIT 1 FOR UPDATE');
            $stmt->execute([$userId, $deviceUuid]);
            $existing = $stmt->fetch();

            if ($existing) {
                if ($existing['status'] !== 'active') {
                    throw new \DomainException('Bu cihaz daha önce kaldırılmış. Hesap panelinden cihaz değişimi yapın.');
                }
                $pdo->prepare('UPDATE user_devices SET device_name=?,last_seen_at=NOW() WHERE id=?')
                    ->execute([$deviceName, $existing['id']]);
                $pdo->commit();
                $existing['device_name'] = $deviceName;
                return $existing;
            }

            $stmt = $pdo->prepare("SELECT * FROM user_devices WHERE user_id=? AND status='active' ORDER BY activated_at DESC,id DESC LIMIT 1 FOR UPDATE");
            $stmt->execute([$userId]);
            $active = $stmt->fetch();
            if ($active) {
                throw new \DomainException('Bu hesap başka bir bilgisayarda etkin. Önce mevcut cihazı hesap panelinden kaldırın.');
            }

            $pdo->prepare("INSERT INTO user_devices(user_id,device_uuid,device_name,status,activated_at,last_seen_at) VALUES(?,?,?,'active',NOW(),NOW())")
                ->execute([$userId, $deviceUuid, $deviceName]);
            $id = (int)$pdo->lastInsertId();
            $pdo->commit();
            return [
                'id'=>$id,
                'user_id'=>$userId,
                'device_uuid'=>$deviceUuid,
                'device_name'=>$deviceName,
                'status'=>'active',
            ];
        } catch (\Throwable $e) {
            if ($pdo->inTransaction()) $pdo->rollBack();
            throw $e;
        }
    }

    public function requireActiveForUser(int $userId, string $deviceUuid): array
    {
        $deviceUuid = $this->validateUuid($deviceUuid);
        $stmt = Database::connection()->prepare("SELECT * FROM user_devices WHERE user_id=? AND device_uuid=? AND status='active' LIMIT 1");
        $stmt->execute([$userId, $deviceUuid]);
        $device = $stmt->fetch();
        if (!$device) throw new \DomainException('Bu cihaz hesap için doğrulanmamış.');
        Database::connection()->prepare('UPDATE user_devices SET last_seen_at=NOW() WHERE id=?')->execute([$device['id']]);
        return $device;
    }

    public function currentForUser(int $userId): ?array
    {
        $stmt = Database::connection()->prepare("SELECT id,device_uuid,device_name,status,activated_at,last_seen_at FROM user_devices WHERE user_id=? AND status='active' ORDER BY activated_at DESC,id DESC LIMIT 1");
        $stmt->execute([$userId]);
        return $stmt->fetch() ?: null;
    }

    public function revokeCurrent(int $userId): void
    {
        $pdo = Database::connection();
        $pdo->beginTransaction();
        try {
            $stmt = $pdo->prepare("SELECT id FROM user_devices WHERE user_id=? AND status='active' ORDER BY activated_at DESC,id DESC LIMIT 1 FOR UPDATE");
            $stmt->execute([$userId]);
            $id = $stmt->fetchColumn();
            if ($id) {
                $pdo->prepare("UPDATE user_devices SET status='revoked',revoked_at=NOW() WHERE id=?")->execute([$id]);
                $pdo->prepare('DELETE FROM api_tokens WHERE device_id=?')->execute([$id]);
            }
            $pdo->commit();
        } catch (\Throwable $e) {
            if ($pdo->inTransaction()) $pdo->rollBack();
            throw $e;
        }
    }
}
