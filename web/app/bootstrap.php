<?php
declare(strict_types=1);

use App\Core\Env;
use App\Core\Session;

const PROJECT_ROOT = __DIR__ . '/../../';
const WEB_ROOT = __DIR__ . '/..';

spl_autoload_register(static function (string $class): void {
    $prefix = 'App\\';
    if (!str_starts_with($class, $prefix)) {
        return;
    }
    $file = __DIR__ . '/' . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
    if (is_file($file)) {
        require $file;
    }
});

Env::load(PROJECT_ROOT . '.env');
date_default_timezone_set(Env::get('APP_TIMEZONE', 'Europe/Istanbul'));

if (PHP_SAPI !== 'cli') {
    Session::start();
}

