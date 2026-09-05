<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use App\Core\Http;
use App\Core\Session;
use PDO;

final class AuthService
{
    public function register(string $email,string $name,string $password): int
    {
        $email=mb_strtolower(trim($email));$name=trim($name);
        if(!filter_var($email,FILTER_VALIDATE_EMAIL))throw new \DomainException('Geçerli e-posta gerekli.');
        if(mb_strlen($name)<2||mb_strlen($name)>100)throw new \DomainException('Görünen ad 2-100 karakter olmalıdır.');
        if(strlen($password)<12||!preg_match('/[A-Z]/',$password)||!preg_match('/[a-z]/',$password)||!preg_match('/\d/',$password))throw new \DomainException('Şifre en az 12 karakter, büyük/küçük harf ve sayı içermelidir.');
        try{Database::connection()->prepare("INSERT INTO users(email,password_hash,display_name,role,release_channel,status) VALUES(?,?,?,'user','stable','active')")->execute([$email,password_hash($password,PASSWORD_DEFAULT),$name]);return(int)Database::connection()->lastInsertId();}catch(\PDOException $error){if($error->getCode()==='23000')throw new \DomainException('Bu e-posta zaten kayıtlı.');throw $error;}
    }

    public function login(string $email, string $password): array
    {
        $pdo = Database::connection();
        $stmt = $pdo->prepare('SELECT id,email,password_hash,display_name,role,release_channel,status FROM users WHERE email = ? LIMIT 1');
        $stmt->execute([mb_strtolower(trim($email))]);
        $user = $stmt->fetch();
        if (!$user || $user['status'] !== 'active' || !password_verify($password, $user['password_hash'])) {
            throw new \DomainException('E-posta veya şifre hatalı.');
        }
        if (password_needs_rehash($user['password_hash'], PASSWORD_DEFAULT)) {
            $pdo->prepare('UPDATE users SET password_hash=? WHERE id=?')->execute([password_hash($password, PASSWORD_DEFAULT), $user['id']]);
        }
        unset($user['password_hash']);
        $pdo->prepare('UPDATE users SET last_login_at=NOW() WHERE id=?')->execute([$user['id']]);
        $user = $this->withEntitlements($user);
        Session::putUser($user);
        return $user;
    }

    public function issueApiToken(int $userId, string $name = 'loader', ?int $deviceId = null): string
    {
        $plain = bin2hex(random_bytes(32));
        Database::connection()->prepare('INSERT INTO api_tokens(user_id,device_id,token_hash,name,expires_at) VALUES(?,?,?,?,DATE_ADD(NOW(), INTERVAL 30 DAY))')
            ->execute([$userId, $deviceId, hash('sha256', $plain), $name]);
        return $plain;
    }

    public function currentUser(): ?array
    {
        $bearer = Http::bearerToken();
        if (!$bearer) {
            return Session::user();
        }

        // Cihaz kimliği token oluşturulurken api_tokens.device_id alanına bağlanır.
        // İstemciden her istekte özel X-* header istemek CORS/preflight sorunlarına
        // yol açtığı için doğrulama tamamen sunucu tarafındaki token-device bağıyla yapılır.
        $stmt = Database::connection()->prepare("SELECT u.id,u.email,u.display_name,u.role,u.release_channel,u.status,d.id device_id,d.device_uuid,d.device_name FROM api_tokens t JOIN users u ON u.id=t.user_id JOIN user_devices d ON d.id=t.device_id AND d.user_id=u.id AND d.status='active' WHERE t.token_hash=? AND t.expires_at>NOW() AND u.status='active' LIMIT 1");
        $stmt->execute([hash('sha256', $bearer)]);
        $user = $stmt->fetch() ?: null;
        if ($user) {
            Database::connection()->prepare('UPDATE api_tokens SET last_used_at=NOW() WHERE token_hash=?')->execute([hash('sha256', $bearer)]);
            Database::connection()->prepare('UPDATE user_devices SET last_seen_at=NOW() WHERE id=?')->execute([$user['device_id']]);
        }
        return $user ? $this->withEntitlements($user) : null;
    }

    public function requireUser(): array
    {
        return $this->currentUser() ?? throw new \RuntimeException('AUTH_REQUIRED');
    }

    public function requireAdmin(): array
    {
        $user = $this->requireUser();
        if (!in_array($user['role'], ['admin','super_admin'], true)) {
            throw new \RuntimeException('ADMIN_REQUIRED');
        }
        return $user;
    }

    public function logout(): void
    {
        if ($token = Http::bearerToken()) {
            Database::connection()->prepare('DELETE FROM api_tokens WHERE token_hash=?')->execute([hash('sha256', $token)]);
        }
        Session::logout();
    }

    public function canAccessPremium(array $user): bool
    {
        $stmt = Database::connection()->prepare("SELECT 1 FROM subscriptions WHERE user_id=? AND status IN ('active','trial') AND (ends_at IS NULL OR ends_at>NOW()) LIMIT 1");
        $stmt->execute([$user['id']]);
        return (bool) $stmt->fetchColumn();
    }

    public function withEntitlements(array $user): array
    {
        $stmt=Database::connection()->prepare("SELECT plan_name,status,starts_at,ends_at FROM subscriptions WHERE user_id=? AND status IN ('active','trial') AND (ends_at IS NULL OR ends_at>NOW()) ORDER BY ends_at IS NULL DESC,ends_at DESC,id DESC LIMIT 1");
        $stmt->execute([$user['id']]);
        $subscription=$stmt->fetch()?:null;
        $user['premium']=$subscription!==null;
        $user['subscription']=$subscription;
        $user['permissions']=[
            'premium_download'=>$subscription!==null,
            'beta_channel'=>in_array($user['release_channel']??'stable',['beta','internal'],true),
            'internal_channel'=>($user['release_channel']??'stable')==='internal',
            'admin'=>in_array($user['role']??'user',['admin','super_admin'],true),
        ];
        return $user;
    }
}
