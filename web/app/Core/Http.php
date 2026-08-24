<?php
declare(strict_types=1);

namespace App\Core;

final class Http
{
    public static function json(array $data, int $status = 200): never
    {
        http_response_code($status);
        header('Content-Type: application/json; charset=utf-8');
        header('X-Content-Type-Options: nosniff');
        echo json_encode($data, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        exit;
    }

    public static function body(): array
    {
        $type = strtolower($_SERVER['CONTENT_TYPE'] ?? '');
        if (str_contains($type, 'application/json')) {
            $decoded = json_decode(file_get_contents('php://input') ?: '{}', true);
            return is_array($decoded) ? $decoded : [];
        }
        return $_POST;
    }

    public static function error(string $message, int $status = 400, array $details = []): never
    {
        self::json(['ok' => false, 'error' => ['message' => $message, 'details' => $details], 'request_id' => self::requestId()], $status);
    }

    public static function requestId(): string
    {
        static $id;
        return $id ??= bin2hex(random_bytes(8));
    }

    public static function bearerToken(): ?string
    {
        $header = $_SERVER['HTTP_AUTHORIZATION'] ?? '';
        return preg_match('/^Bearer\s+(.+)$/i', $header, $m) ? trim($m[1]) : null;
    }
}

