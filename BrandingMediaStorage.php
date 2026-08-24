<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Env;
use App\Core\Http;

final class BrandingMediaStorage
{
    private const IMAGE_MIMES=[
        'jpg'=>'image/jpeg',
        'jpeg'=>'image/jpeg',
        'png'=>'image/png',
        'webp'=>'image/webp',
    ];
    private const VIDEO_MIMES=[
        'mp4'=>'video/mp4',
        'webm'=>'video/webm',
    ];

    public function storeUpload(array $file,string $kind): array
    {
        if(!in_array($kind,['image','video'],true))throw new \DomainException('Medya türü geçersiz.');
        if(($file['error']??UPLOAD_ERR_NO_FILE)!==UPLOAD_ERR_OK||!is_uploaded_file((string)($file['tmp_name']??'')))throw new \DomainException('Branding medyası yüklenemedi.');
        $size=(int)($file['size']??0);
        $limit=$kind==='image'?Env::int('MAX_IMAGE_UPLOAD_SIZE',15728640):Env::int('MAX_VIDEO_UPLOAD_SIZE',262144000);
        if($size<1||$size>$limit)throw new \DomainException($kind==='image'?'Arka plan resmi boyut sınırını aşıyor.':'Arka plan videosu boyut sınırını aşıyor.');
        $extension=strtolower(pathinfo((string)($file['name']??''),PATHINFO_EXTENSION));
        $allowed=$kind==='image'?self::IMAGE_MIMES:self::VIDEO_MIMES;
        $mime=(new \finfo(FILEINFO_MIME_TYPE))->file((string)$file['tmp_name']);
        if(!isset($allowed[$extension])||!hash_equals($allowed[$extension],(string)$mime))throw new \DomainException('Dosya uzantısı ve gerçek MIME türü eşleşmiyor.');
        if($kind==='image'){
            $info=@getimagesize((string)$file['tmp_name']);
            if(!$info||($info[0]*$info[1])>60000000||($info['mime']??'')!==$mime)throw new \DomainException('Arka plan resmi çözümlenemedi veya boyutları güvenli değil.');
        }else{
            $header=(string)file_get_contents((string)$file['tmp_name'],false,null,0,16);
            if($extension==='mp4'&&(strlen($header)<12||substr($header,4,4)!=='ftyp'))throw new \DomainException('MP4 container imzası geçersiz.');
            if($extension==='webm'&&!str_starts_with($header,"\x1A\x45\xDF\xA3"))throw new \DomainException('WebM container imzası geçersiz.');
        }
        $directory=$this->storageDir();
        if(!is_dir($directory)&&!mkdir($directory,0750,true)&&!is_dir($directory))throw new \RuntimeException('Branding media storage oluşturulamadı.');
        $storageName=bin2hex(random_bytes(24)).'.'.$extension;
        $target=$directory.DIRECTORY_SEPARATOR.$storageName;
        if(!move_uploaded_file((string)$file['tmp_name'],$target))throw new \RuntimeException('Branding medyası private storage alanına yazılamadı.');
        return ['storage_name'=>$storageName,'url'=>$this->publicUrl($storageName),'sha256'=>hash_file('sha256',$target),'size_bytes'=>filesize($target),'mime_type'=>$mime];
    }

    public function deleteManagedUrl(?string $url): void
    {
        if(!$url||!preg_match('#^/media/branding/([a-f0-9]{48}\.(?:jpe?g|png|webp|mp4|webm))$#i',$url,$match))return;
        $path=$this->path($match[1]);if(is_file($path))unlink($path);
    }

    public function publicUrl(string $storageName): string
    {
        return '/media/branding/'.basename($storageName);
    }

    public function stream(string $storageName): never
    {
        if(!preg_match('/^[a-f0-9]{48}\.(?:jpe?g|png|webp|mp4|webm)$/i',$storageName))Http::error('Medya bulunamadı.',404);
        $path=$this->path($storageName);if(!is_file($path))Http::error('Medya bulunamadı.',404);
        $extension=strtolower(pathinfo($storageName,PATHINFO_EXTENSION));$mime=(self::IMAGE_MIMES+self::VIDEO_MIMES)[$extension]??'application/octet-stream';$size=filesize($path);$start=0;$end=$size-1;$status=200;
        header('Content-Type: '.$mime);header('Accept-Ranges: bytes');header('Cache-Control: public, max-age=31536000, immutable');header('ETag: "'.hash_file('sha256',$path).'"');header('X-Content-Type-Options: nosniff');
        $range=$_SERVER['HTTP_RANGE']??'';
        if($range!==''){
            if(!preg_match('/^bytes=(\d*)-(\d*)$/',$range,$parts)||($parts[1]===''&&$parts[2]==='')){header('Content-Range: bytes */'.$size);http_response_code(416);exit;}
            if($parts[1]===''){$length=min((int)$parts[2],$size);$start=$size-$length;}else{$start=(int)$parts[1];}
            if($parts[2]!==''&&$parts[1]!=='')$end=min((int)$parts[2],$size-1);
            if($start<0||$start>$end||$start>=$size){header('Content-Range: bytes */'.$size);http_response_code(416);exit;}
            $status=206;header("Content-Range: bytes {$start}-{$end}/{$size}");
        }
        http_response_code($status);$length=$end-$start+1;header('Content-Length: '.$length);
        $handle=fopen($path,'rb');if($handle===false)Http::error('Medya açılamadı.',500);fseek($handle,$start);$remaining=$length;while($remaining>0&&!feof($handle)){$chunk=fread($handle,min(1048576,$remaining));if($chunk===false)break;echo $chunk;$remaining-=strlen($chunk);if(connection_aborted())break;}fclose($handle);exit;
    }

    /** Storage GC ve orphan tarama için mutlak dizin yolu. */
    public function directory(): string
    {
        return $this->storageDir();
    }

    /** /media/branding/<ad> public URL'sini diskteki mutlak yola çevirir. */
    public function absolutePathFromUrl(?string $url): ?string
    {
        if (!$url || !preg_match('#^/media/branding/([a-f0-9]{48}\.(?:jpe?g|png|webp|mp4|webm))$#i', $url, $match)) {
            return null;
        }
        return $this->path($match[1]);
    }

    private function path(string $storageName): string
    {
        return $this->storageDir().DIRECTORY_SEPARATOR.basename($storageName);
    }

    private function storageDir(): string
    {
        $configured=Env::get('BRANDING_MEDIA_STORAGE_PATH','storage/media/branding');
        return preg_match('~^[a-zA-Z]:[\\\\/]~',$configured)?$configured:WEB_ROOT.DIRECTORY_SEPARATOR.str_replace(['/','\\'],DIRECTORY_SEPARATOR,$configured);
    }
}

