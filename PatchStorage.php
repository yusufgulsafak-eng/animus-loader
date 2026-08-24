<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Env;
use App\Core\PathGuard;
use ZipArchive;

final class PatchStorage
{
    public function storeUpload(array $file): array
    {
        if (($file['error'] ?? UPLOAD_ERR_NO_FILE) !== UPLOAD_ERR_OK || !is_uploaded_file($file['tmp_name'])) throw new \DomainException('ZIP yüklemesi başarısız.');
        if (($file['size'] ?? 0) < 1 || $file['size'] > Env::int('UPLOAD_MAX_BYTES',1073741824)) throw new \DomainException('Dosya boyutu izin verilen aralıkta değil.');
        $mime=(new \finfo(FILEINFO_MIME_TYPE))->file($file['tmp_name']);
        if (!in_array($mime,['application/zip','application/x-zip-compressed','application/octet-stream'],true) || strtolower(pathinfo($file['name'],PATHINFO_EXTENSION))!=='zip') throw new \DomainException('Yalnızca geçerli ZIP arşivi yüklenebilir.');
        $tree=$this->inspectZip($file['tmp_name']);
        $name=bin2hex(random_bytes(24)).'.zip';
        $dir=$this->storageDir(); if(!is_dir($dir) && !mkdir($dir,0750,true) && !is_dir($dir)) throw new \RuntimeException('Patch storage oluşturulamadı.');
        $target=$dir.DIRECTORY_SEPARATOR.$name;
        if(!move_uploaded_file($file['tmp_name'],$target)) throw new \RuntimeException('Patch storage yazılamadı.');
        return ['storage_name'=>$name,'original_name'=>basename($file['name']),'mime_type'=>$mime,'sha256'=>hash_file('sha256',$target),'size_bytes'=>filesize($target),'file_tree'=>$tree];
    }

    public function inspectZip(string $path): array
    {
        $zip=new ZipArchive(); if($zip->open($path)!==true) throw new \DomainException('ZIP arşivi okunamıyor.');
        $tree=[]; $total=0;
        try {
            if($zip->numFiles>100000) throw new \DomainException('ZIP çok fazla dosya içeriyor.');
            for($i=0;$i<$zip->numFiles;$i++) {
                $stat=$zip->statIndex($i); $name=rtrim(str_replace('\\','/',$stat['name'] ?? ''),'/');
                if($name==='' || !PathGuard::isSafeRelative($name)) throw new \DomainException('ZIP içinde güvenli olmayan yol: '.($stat['name'] ?? ''));
                if(($stat['size'] ?? 0)>2147483648) throw new \DomainException('ZIP entry boyutu sınırı aşıyor.');
                $total += (int)($stat['size'] ?? 0);
                if($total > 21474836480) throw new \DomainException('ZIP açılmış toplam boyut sınırı aşıyor.');
                $opsys=0; $attributes=0;
                if($zip->getExternalAttributesIndex($i,$opsys,$attributes) && (($attributes >> 16) & 0170000) === 0120000) {
                    throw new \DomainException('ZIP symbolic link girdisi içeremez.');
                }
                $tree[]=['path'=>$name,'size'=>(int)($stat['size'] ?? 0),'directory'=>str_ends_with($stat['name'],'/')];
            }
        } finally { $zip->close(); }
        return $tree;
    }

    /** Storage GC ve orphan tarama için mutlak dizin yolu. */
    public function directory(): string { return $this->storageDir(); }

    public function path(string $storageName): string { return $this->storageDir().DIRECTORY_SEPARATOR.basename($storageName); }
    private function storageDir(): string { $configured=Env::get('PATCH_STORAGE_PATH','storage/patches'); return preg_match('~^[a-zA-Z]:[\\\\/]~',$configured) ? $configured : WEB_ROOT.DIRECTORY_SEPARATOR.str_replace(['/', '\\'],DIRECTORY_SEPARATOR,$configured); }
}
