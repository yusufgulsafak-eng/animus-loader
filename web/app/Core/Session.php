<?php
declare(strict_types=1);

namespace App\Core;

final class Session
{
    public static function start(): void
    {
        if (session_status() === PHP_SESSION_ACTIVE) {
            return;
        }
        session_name(Env::get('SESSION_NAME', 'turkce_yama_session'));
        session_set_cookie_params([
            'lifetime' => 0,
            'path' => '/',
            'secure' => Env::bool('SESSION_SECURE', true),
            'httponly' => true,
            'samesite' => 'Lax',
        ]);
        ini_set('session.use_strict_mode', '1');
        ini_set('session.use_only_cookies', '1');
        session_start();
    }

    public static function user(): ?array { return $_SESSION['user'] ?? null; }
    public static function putUser(array $user): void { session_regenerate_id(true); $_SESSION['user'] = $user; }
    public static function logout(): void { $_SESSION = []; session_regenerate_id(true); }
    public static function isAdmin(): bool { return in_array(self::user()['role'] ?? '', ['admin', 'super_admin'], true); }
}

