<?php
declare(strict_types=1);

// Fixture IDs come from the existing example. This is never a published game.
function shadows_test_manifest(): array
{
    $m = json_decode(file_get_contents(__DIR__.'/../examples/manifest-v1.json'), true, 512, JSON_THROW_ON_ERROR);
    $m['game']['slug'] = 'assassins-creed-shadows';
    $m['detection']['executable'] = 'ACShadows.exe';
    $m['detection']['process_name'] = 'ACShadows.exe';
    $m['detection']['required_files'] = ['DataPC_boot.forge','DataPC_boot_patch_01.forge','DataPC_boot_patch_02.forge','dlc_20/DataPC_boot_20_dlc.forge','dlc_20/DataPC_boot_20_dlc_patch_01.forge'];
    $m['patch']['minimum_loader_version'] = '0.1.5';
    $m['install_actions'] = [[
        'id'=>'762b942d-58d8-4524-beb7-ae37cce1a6d7',
        'type'=>'INSTALL_SHADOWS',
        'source'=>'MANIFEST_SHA256.txt',
        'destination'=>'DataPC_boot.forge',
        'backup'=>true,
        'expected_sha256'=>'ee397ca77e96e0a1c53fff3cf0d26cd58b64a99cdad8c69d9ef34014d87ff6e4',
        'options'=>new stdClass(),
    ]];
    return $m;
}

if (realpath($_SERVER['SCRIPT_FILENAME'] ?? '') === __FILE__) {
    echo json_encode(shadows_test_manifest(), JSON_PRETTY_PRINT|JSON_UNESCAPED_SLASHES|JSON_THROW_ON_ERROR), "\n";
}
