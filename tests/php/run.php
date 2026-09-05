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
$manifest=json_decode(file_get_contents(__DIR__.'/../../examples/manifest-v1.json'),true,512,JSON_THROW_ON_ERROR);
$action=$manifest['install_actions'][0];
$action['type']='APPEND_FAT_DAT';
$action['source']='ceviri.bin';
$action['destination']='data_final/pc/patch.dat';
$action['backup']=true;
$action['options']=['fat_path'=>'data_final/pc/patch.fat','fat_entry_hash'=>'F2BDFD295DE3F8F3',
    'base_dat_sha256'=>str_repeat('a',64),'base_fat_sha256'=>str_repeat('b',64),
    'payload_sha256'=>str_repeat('c',64),'alignment'=>8,'compression'=>'none'];
$manifest['install_actions']=[$action];
$manifest['patch']['minimum_loader_version']='0.1.1';
$manifest['detection']['process_name']='FarCry5.exe';
expect((new ManifestValidator())->validate($manifest)===[],'native FAT/DAT manifest validates');
foreach (['../patch.fat','C:/patch.fat','data_final/pc/wrong.fat','data_final/pc/patch.fat '] as $bad) {
    $m=$manifest; $m['install_actions'][0]['options']['fat_path']=$bad;
    expect((new ManifestValidator())->validate($m)!==[],'invalid FAT path rejected');
}
foreach (['base_dat_sha256','base_fat_sha256','payload_sha256','fat_entry_hash','alignment','compression'] as $key) {
    $m=$manifest; unset($m['install_actions'][0]['options'][$key]);
    expect((new ManifestValidator())->validate($m)!==[],'missing option rejected: '.$key);
}
$m=$manifest; $m['install_actions'][]=$action;
expect((new ManifestValidator())->validate($m)!==[],'mixed archive actions rejected');
$m=$manifest; $m['patch']['minimum_loader_version']='0.1.0';
expect((new ManifestValidator())->validate($m)!==[],'old loader gate rejected');
$m=$manifest; $m['install_actions'][0]['options']['execute']='FC5_Kur.exe';
expect((new ManifestValidator())->validate($m)!==[],'unknown executable option rejected');
echo "PASS: {$tests} PHP security/manifest assertions.\n";


