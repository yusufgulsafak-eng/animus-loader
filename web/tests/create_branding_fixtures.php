<?php
declare(strict_types=1);

$directory=dirname(__DIR__,2).'/.local/branding-tests';
if(!is_dir($directory)&&!mkdir($directory,0750,true)&&!is_dir($directory))throw new RuntimeException('Fixture klasörü oluşturulamadı.');
$image=imagecreatetruecolor(64,36);$background=imagecolorallocate($image,40,18,65);imagefill($image,0,0,$background);
imagejpeg($image,$directory.'/valid.jpg',90);imagepng($image,$directory.'/valid.png');imagewebp($image,$directory.'/valid.webp',90);imagedestroy($image);
file_put_contents($directory.'/fake.jpg','<?php echo 1; ?>');
file_put_contents($directory.'/blocked.exe',"MZ\0\0");
$oversized=fopen($directory.'/oversized.mp4','wb');fwrite($oversized,"\0\0\0\x18ftypisom\0\0\0\0isomiso2");ftruncate($oversized,1048577);fclose($oversized);
foreach(glob($directory.'/*') as $file)echo basename($file).' '.filesize($file).PHP_EOL;
