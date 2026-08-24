<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Env;

final class ImageStorage
{
    private const MIME_EXTENSIONS = [
        'image/jpeg' => 'jpg',
        'image/png' => 'png',
        'image/webp' => 'webp',
    ];

    public function store(array $file, string $kind): string
    {
        if (!in_array($kind, ['cover','banner','icon'], true)) throw new \DomainException('Görsel türü geçersiz.');
        if (($file['error'] ?? UPLOAD_ERR_NO_FILE) !== UPLOAD_ERR_OK || !is_uploaded_file((string)($file['tmp_name'] ?? ''))) throw new \DomainException('Görsel yüklemesi başarısız.');
        $size=(int)($file['size']??0);$limit=Env::int('IMAGE_UPLOAD_MAX_BYTES',15*1024*1024);
        if($size<1||$size>$limit)throw new \DomainException('Görsel boyutu izin verilen aralıkta değil.');
        $mime=(new \finfo(FILEINFO_MIME_TYPE))->file($file['tmp_name']);
        $extension=self::MIME_EXTENSIONS[$mime]??null;
        $clientExtension=strtolower(pathinfo((string)($file['name']??''),PATHINFO_EXTENSION));
        if(!$extension||!in_array($clientExtension,['jpg','jpeg','png','webp'],true))throw new \DomainException('Yalnız JPG, PNG veya WebP yüklenebilir.');
        $info=@getimagesize($file['tmp_name']);
        if(!$info||$info[0]<1||$info[1]<1||($info[0]*$info[1])>50000000)throw new \DomainException('Görsel içeriği veya çözünürlüğü geçersiz.');

        $dir=WEB_ROOT.DIRECTORY_SEPARATOR.'public'.DIRECTORY_SEPARATOR.'uploads'.DIRECTORY_SEPARATOR.'games';
        if(!is_dir($dir)&&!mkdir($dir,0750,true)&&!is_dir($dir))throw new \RuntimeException('Görsel storage oluşturulamadı.');
        $name=$kind.'-'.bin2hex(random_bytes(20)).'.'.$extension;
        $target=$dir.DIRECTORY_SEPARATOR.$name;
        if(!$this->optimize($file['tmp_name'],$target,$mime,$kind)&&!move_uploaded_file($file['tmp_name'],$target))throw new \RuntimeException('Görsel kaydedilemedi.');
        return '/uploads/games/'.$name;
    }

    public function deleteManaged(?string $publicPath): void
    {
        if(!$publicPath||!str_starts_with($publicPath,'/uploads/games/'))return;
        $name=basename($publicPath);
        if(!preg_match('/^(cover|banner|icon)-[a-f0-9]{40}\.(jpg|png|webp)$/',$name))return;
        $path=WEB_ROOT.DIRECTORY_SEPARATOR.'public'.DIRECTORY_SEPARATOR.'uploads'.DIRECTORY_SEPARATOR.'games'.DIRECTORY_SEPARATOR.$name;
        if(is_file($path))unlink($path);
    }

    private function optimize(string $source,string $target,string $mime,string $kind): bool
    {
        if(!function_exists('imagecreatetruecolor'))return false;
        $create=match($mime){'image/jpeg'=>'imagecreatefromjpeg','image/png'=>'imagecreatefrompng','image/webp'=>'imagecreatefromwebp',default=>null};
        if(!$create||!function_exists($create))return false;
        $input=@$create($source);if(!$input)return false;
        $width=imagesx($input);$height=imagesy($input);$max=match($kind){'icon'=>512,'cover'=>1440,default=>2560};$ratio=min(1,$max/max($width,$height));
        $newWidth=max(1,(int)round($width*$ratio));$newHeight=max(1,(int)round($height*$ratio));$output=imagecreatetruecolor($newWidth,$newHeight);
        imagealphablending($output,false);imagesavealpha($output,true);$transparent=imagecolorallocatealpha($output,0,0,0,127);imagefill($output,0,0,$transparent);
        imagecopyresampled($output,$input,0,0,0,0,$newWidth,$newHeight,$width,$height);
        $saved=match($mime){'image/jpeg'=>imagejpeg($output,$target,86),'image/png'=>imagepng($output,$target,7),'image/webp'=>imagewebp($output,$target,84),default=>false};
        imagedestroy($input);imagedestroy($output);return(bool)$saved;
    }
}
