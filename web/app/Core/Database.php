<?php
declare(strict_types=1);

namespace App\Core;

use PDO;
use PDOException;
use RuntimeException;

final class Database
{
    private static ?PDO $pdo = null;

    public static function connection(): PDO
    {
        if (self::$pdo !== null) {
            return self::$pdo;
        }

        $host = Env::get('DB_HOST', 'localhost');
        $port = Env::get('DB_PORT', '3306');
        $database = Env::get('DB_DATABASE', Env::get('DB_NAME', ''));
        $username = Env::get('DB_USERNAME', Env::get('DB_USER', ''));
        $password = Env::get('DB_PASSWORD', Env::get('DB_PASS', ''));

        if ($database === '' || $username === '') {
            throw new RuntimeException(
                'Veritabanı ayarları eksik: DB_DATABASE veya DB_USERNAME boş.'
            );
        }

        $dsn = sprintf(
            'mysql:host=%s;port=%s;dbname=%s;charset=utf8mb4',
            $host,
            $port,
            $database
        );

        try {
            self::$pdo = new PDO(
                $dsn,
                $username,
                $password,
                [
                    PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
                    PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
                    PDO::ATTR_EMULATE_PREPARES => false,
                    PDO::ATTR_STRINGIFY_FETCHES => false,
                ]
            );

            return self::$pdo;

        } catch (PDOException $e) {
            $debug = Env::get('APP_DEBUG', 'false') === 'true';

            if ($debug) {
                throw new RuntimeException(
                    'Veritabanı bağlantısı kurulamadı: ' . $e->getMessage(),
                    0,
                    $e
                );
            }

            throw new RuntimeException(
                'Veritabanı bağlantısı kurulamadı.',
                0,
                $e
            );
        }
    }
}