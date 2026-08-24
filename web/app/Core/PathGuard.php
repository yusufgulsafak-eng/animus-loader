<?php
declare(strict_types=1);

namespace App\Core;

final class PathGuard
{
    public static function isSafeRelative(string $path): bool
    {
        $path = str_replace('\\', '/', trim($path));
        if ($path === '' || str_starts_with($path, '/') || preg_match('/^[a-zA-Z]:/', $path) || str_starts_with($path, '//')) {
            return false;
        }
        foreach (explode('/', $path) as $part) {
            if ($part === '' || $part === '.' || $part === '..' || str_contains($part, "\0")) {
                return false;
            }
        }
        return true;
    }
}

