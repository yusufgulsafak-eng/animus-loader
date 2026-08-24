<?php
declare(strict_types=1);

require __DIR__ . '/../../web/app/bootstrap.php';

use App\Core\Database;
use App\Services\PatchStorage;

if (PHP_SAPI !== 'cli') {
    fwrite(STDERR, "Bu fixture yalnız CLI üzerinden çalıştırılır.\n");
    exit(1);
}

function removeFixtureDirectory(string $path): void
{
    $normalized = str_replace('\\', '/', $path);
    if (!str_contains($normalized . '/', '/.local/fixtures/')) {
        throw new RuntimeException('Fixture dışı dizin silme girişimi engellendi.');
    }
    if (!is_dir($path)) {
        return;
    }
    $iterator = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($path, FilesystemIterator::SKIP_DOTS),
        RecursiveIteratorIterator::CHILD_FIRST
    );
    foreach ($iterator as $entry) {
        $entry->isDir() ? rmdir($entry->getPathname()) : unlink($entry->getPathname());
    }
    rmdir($path);
}

function uuidV4(): string
{
    $data = random_bytes(16);
    $data[6] = chr((ord($data[6]) & 0x0f) | 0x40);
    $data[8] = chr((ord($data[8]) & 0x3f) | 0x80);
    return vsprintf('%s%s-%s-%s-%s-%s%s%s', str_split(bin2hex($data), 4));
}

$projectRoot = realpath(PROJECT_ROOT) ?: PROJECT_ROOT;
$fixtureRoot = rtrim($projectRoot, '/\\') . '/.local/fixtures';
$gameRoot = $fixtureRoot . '/Patch Engine Test Game';
$archiveSource = $fixtureRoot . '/patch-engine-test.zip';
removeFixtureDirectory($fixtureRoot);
mkdir($gameRoot . '/Data', 0750, true);
file_put_contents($gameRoot . '/Game.exe', "PATCH ENGINE TEST EXECUTABLE\n");
file_put_contents($gameRoot . '/Data/original.txt', "ORIGINAL\n");

$zip = new ZipArchive();
if ($zip->open($archiveSource, ZipArchive::CREATE | ZipArchive::OVERWRITE) !== true) {
    throw new RuntimeException('Fixture ZIP oluşturulamadı.');
}
$zip->addFromString('files/original.txt', "TURKISH PATCH\n");
$zip->addFromString('files/turkish.txt', "TURKISH FILE\n");
$zip->close();

$storage = new PatchStorage();
$tree = $storage->inspectZip($archiveSource);
$storageName = bin2hex(random_bytes(24)) . '.zip';
$storedArchive = $storage->path($storageName);
if (!is_dir(dirname($storedArchive))) {
    mkdir(dirname($storedArchive), 0750, true);
}
if (!copy($archiveSource, $storedArchive)) {
    throw new RuntimeException('Fixture ZIP private storage alanına kopyalanamadı.');
}

$pdo = Database::connection();
$oldStorage = $pdo->prepare(
    'SELECT pa.storage_name FROM games g
     JOIN patches p ON p.game_id=g.id
     JOIN patch_versions pv ON pv.patch_id=p.id
     JOIN patch_archives pa ON pa.patch_version_id=pv.id
     WHERE g.slug=?'
);
$oldStorage->execute(['patch-engine-test-game']);
$oldNames = $oldStorage->fetchAll(PDO::FETCH_COLUMN);
$pdo->prepare('DELETE FROM games WHERE slug=?')->execute(['patch-engine-test-game']);
foreach ($oldNames as $name) {
    $oldPath = $storage->path((string)$name);
    if (is_file($oldPath)) {
        unlink($oldPath);
    }
}

$pdo->beginTransaction();
try {
    $pdo->prepare(
        'INSERT INTO games
        (name,slug,short_description,description,cover_path,banner_path,local_cover_path,local_banner_path,
         executable,process_name,access_type,translation_percent,minimum_loader_version,supported_stores,is_active)
         VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,0)'
    )->execute([
        'Patch Engine Test Game',
        'patch-engine-test-game',
        'Yalnız otomatik entegrasyon testlerinde kullanılan pasif fixture.',
        'Gerçek oyun dosyalarına dokunmadan download, hash, backup, install ve restore zincirini doğrular.',
        '/assets/placeholders/cover-generic.svg',
        '/assets/placeholders/banner-generic.svg',
        '/assets/placeholders/cover-generic.svg',
        '/assets/placeholders/banner-generic.svg',
        'Game.exe',
        'PatchEngineFixtureThatNeverRuns.exe',
        'free',
        100,
        '0.1.0',
        json_encode(['manual'], JSON_THROW_ON_ERROR),
    ]);
    $gameId = (int)$pdo->lastInsertId();
    $pdo->prepare(
        "INSERT INTO game_detection_rules(game_id,provider,rule_type,rule_value,sort_order,is_required)
         VALUES(?,'manual','required_file','Game.exe',10,1)"
    )->execute([$gameId]);
    $pdo->prepare('INSERT INTO patches(game_id,name,description) VALUES(?,?,?)')
        ->execute([$gameId, 'Fixture Türkçe Yama', 'Otomatik entegrasyon fixture patchi.']);
    $patchId = (int)$pdo->lastInsertId();
    $pdo->prepare(
        "INSERT INTO patch_versions
        (patch_id,version,game_version,changelog,minimum_loader_version,status,channel,mandatory_update,access_type,schema_version,published_at)
        VALUES(?,'1.0.0','fixture-1','Replace original.txt and create turkish.txt','0.1.0','PUBLISHED','stable',0,'free',1,NOW())"
    )->execute([$patchId]);
    $versionId = (int)$pdo->lastInsertId();
    $sha256 = hash_file('sha256', $storedArchive);
    $size = filesize($storedArchive);
    $pdo->prepare(
        'INSERT INTO patch_archives
        (patch_version_id,storage_name,original_name,mime_type,sha256,size_bytes,file_tree)
        VALUES(?,?,?,?,?,?,?)'
    )->execute([
        $versionId,
        $storageName,
        'patch-engine-test.zip',
        'application/zip',
        $sha256,
        $size,
        json_encode($tree, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR),
    ]);
    $actions = [
        [uuidV4(), 'REPLACE_FILE', 'files/original.txt', 'Data/original.txt', hash('sha256', "TURKISH PATCH\n"), 10],
        [uuidV4(), 'COPY_FILE', 'files/turkish.txt', 'Data/turkish.txt', hash('sha256', "TURKISH FILE\n"), 20],
    ];
    $action = $pdo->prepare(
        'INSERT INTO patch_install_actions
        (patch_version_id,action_uuid,action_type,source_path,destination_path,backup_enabled,expected_sha256,sort_order,options_json)
        VALUES(?,?,?,?,?,1,?,?,?)'
    );
    foreach ($actions as [$uuid, $type, $source, $destination, $expected, $order]) {
        $action->execute([$versionId, $uuid, $type, $source, $destination, $expected, $order, '{}']);
    }
    $pdo->prepare(
        "INSERT INTO patch_release_channels(patch_id,channel,active_patch_version_id)
         VALUES(?,'stable',?)"
    )->execute([$patchId, $versionId]);
    $pdo->commit();
} catch (Throwable $error) {
    if ($pdo->inTransaction()) {
        $pdo->rollBack();
    }
    if (is_file($storedArchive)) {
        unlink($storedArchive);
    }
    throw $error;
}

echo json_encode([
    'game_id' => $gameId,
    'patch_version_id' => $versionId,
    'game_root' => $gameRoot,
    'archive_sha256' => $sha256,
    'archive_size' => $size,
], JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR) . PHP_EOL;
