<?php
declare(strict_types=1);

namespace App\Core;

final class Cors
{
    public static function handle(): void
    {
        $origin = trim((string)($_SERVER['HTTP_ORIGIN'] ?? ''));
        if ($origin === '') {
            return;
        }

        $allowed = array_values(array_filter(array_map(
            static fn(string $value): string => rtrim(trim($value), '/'),
            explode(',', Env::get('CORS_ORIGINS', '') ?? '')
        )));
        $normalized = rtrim($origin, '/');
        $isAllowed = in_array($normalized, $allowed, true);

        header('Vary: Origin');
        if ($isAllowed) {
            header('Access-Control-Allow-Origin: ' . $origin);
            header('Access-Control-Allow-Credentials: true');
            header('Access-Control-Allow-Headers: Accept, Authorization, Content-Type, X-CSRF-Token');
            header('Access-Control-Allow-Methods: GET, POST, OPTIONS');
            header('Access-Control-Max-Age: 600');
        }

        if (strtoupper($_SERVER['REQUEST_METHOD'] ?? 'GET') !== 'OPTIONS') {
            return;
        }
        if (!$isAllowed) {
            Http::error('Bu origin için erişim izni yok.', 403);
        }
        http_response_code(204);
        exit;
    }
}
