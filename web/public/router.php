<?php
declare(strict_types=1);

$uri = rawurldecode(parse_url($_SERVER['REQUEST_URI'] ?? '/', PHP_URL_PATH) ?: '/');
$publicRoot = realpath(__DIR__);
$candidate = realpath(__DIR__ . DIRECTORY_SEPARATOR . ltrim($uri, '/'));

if (
    $uri !== '/'
    && $publicRoot !== false
    && $candidate !== false
    && str_starts_with($candidate, $publicRoot . DIRECTORY_SEPARATOR)
    && is_file($candidate)
) {
    return false;
}

require __DIR__ . '/index.php';
