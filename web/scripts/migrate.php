<?php
declare(strict_types=1);
require __DIR__ . '/../app/bootstrap.php';
use App\Core\Database;
if(PHP_SAPI!=='cli'){fwrite(STDERR,"Bu araç yalnız CLI üzerinden çalıştırılır.\n");exit(1);}
$pdo=Database::connection();
$pdo->exec("CREATE TABLE IF NOT EXISTS schema_migrations (
    migration VARCHAR(190) PRIMARY KEY,
    checksum CHAR(64) NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci");
$applied=$pdo->query('SELECT migration FROM schema_migrations')->fetchAll(PDO::FETCH_COLUMN);
$files=glob(__DIR__.'/../database/*.sql')?:[];
sort($files);
foreach($files as $file){
    $name=basename($file);
    if(in_array($name,$applied,true)){echo 'Atlanıyor: '.$name.PHP_EOL;continue;}
    $sql=file_get_contents($file);
    if($sql===false)throw new RuntimeException('Migration okunamadı: '.$name);
    echo 'Uygulanıyor: '.$name.PHP_EOL;
    $pdo->exec($sql);
    $pdo->prepare('INSERT INTO schema_migrations(migration,checksum) VALUES(?,?)')->execute([$name,hash('sha256',$sql)]);
}
echo "Migration ve seed tamamlandı.\n";
