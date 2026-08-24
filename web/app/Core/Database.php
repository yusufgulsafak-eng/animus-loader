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
        $database = Env::get('DB_DATABASE', Env::get('DB_NAME', 'turkce_yama'));
        $username = Env::get('DB_USERNAME', Env::get('DB_USER', ''));
        $password = Env::get('DB_PASSWORD', Env::get('DB_PASS', ''));
        $dsn = sprintf('mysql:host=%s;port=%s;dbname=%s;charset=utf8mb4',
            Env::get('DB_HOST', '127.0.0.1'), Env::get('DB_PORT', '3306'), $database);
        try {
            self::$pdo = new PDO($dsn, $username, $password, [
                PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
                PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
                PDO::ATTR_EMULATE_PREPARES => false,
                PDO::ATTR_STRINGIFY_FETCHES => false,
            ]);
        } catch (PDOException $e) {
            throw new RuntimeException('Veritabanı bağlantısı kurulamadı.', 0, $e);
        }
        return self::$pdo;
    }
}
