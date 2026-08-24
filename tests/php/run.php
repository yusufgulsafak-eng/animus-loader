<?php
declare(strict_types=1);
require __DIR__.'/../../web/app/bootstrap.php';
use App\Core\PathGuard;
use App\Services\ManifestValidator;
$tests=0;
function expect(bool $condition,string $message):void{global $tests;$tests++;if(!$condition){fwrite(STDERR,"FAIL: {$message}\n");exit(1);}}
expect(PathGuard::isSafeRelative('Content/Paks/file.pak'),'nested relative path accepted');
expect(!PathGuard::isSafeRelative('../Windows/System32'),'parent traversal rejected');
expect(!PathGuard::isSafeRelative('C:\\Windows\\file'),'drive path rejected');
expect(!PathGuard::isSafeRelative('\\\\server\\share'),'UNC path rejected');
$manifest=json_decode(file_get_contents(__DIR__.'/../../examples/manifest-v1.json'),true,512,JSON_THROW_ON_ERROR);
$errors=(new ManifestValidator())->validate($manifest);
expect($errors===[],'example manifest validates: '.implode(', ',$errors));
$manifest['install_actions'][0]['destination']='../../escape';
expect((new ManifestValidator())->validate($manifest)!==[],'unsafe manifest path rejected');
echo "PASS: {$tests} PHP security/manifest assertions.\n";

