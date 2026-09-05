<?php
declare(strict_types=1);

namespace App\Core;

final class RateLimiter
{
    public static function attempt(string $bucket, int $limit = 0, int $window = 60): bool
    {
        $limit = $limit ?: Env::int('RATE_LIMIT_PER_MINUTE', 60);
        $key = hash('sha256', $bucket . '|' . ($_SERVER['REMOTE_ADDR'] ?? 'unknown'));
        $pdo = Database::connection();

        $pdo->prepare('DELETE FROM rate_limits WHERE expires_at < NOW()')->execute();

        $stmt = $pdo->prepare('SELECT hits, expires_at FROM rate_limits WHERE bucket_key = ?');
        $stmt->execute([$key]);
        $row = $stmt->fetch();

        if (!$row) {
            $pdo->prepare(
                'INSERT INTO rate_limits (bucket_key, hits, expires_at) VALUES (?, 1, DATE_ADD(NOW(), INTERVAL ? SECOND))'
            )->execute([$key, $window]);
            return true;
        }

        if ((int) $row['hits'] >= $limit) {
            return false;
        }

        $pdo->prepare('UPDATE rate_limits SET hits = hits + 1 WHERE bucket_key = ?')->execute([$key]);
        return true;
    }

    public static function enforce(string $bucket, int $limit = 0, int $window = 60): void
    {
        if (!self::attempt($bucket, $limit, $window)) {
            Http::error('Çok fazla istek. Lütfen kısa süre sonra tekrar deneyin.', 429);
        }
    }
}
