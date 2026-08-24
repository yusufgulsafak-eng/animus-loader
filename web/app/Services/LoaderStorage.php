<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Env;

final class LoaderStorage
{
    private const EXTENSIONS = ['exe','msi','msix','zip'];
    private const MIMES = [
        'application/octet-stream',
        'application/x-msdownload',
        'application/x-msi',
        'application/msi',
        'application/zip',
        'application/x-zip-compressed',
    ];

    public function storeUpload(array $file): array
    {
        if (($file['error'] ?? UPLOAD_ERR_NO_FILE) !== UPLOAD_ERR_OK || !is_uploaded_file((string)($file['tmp_name'] ?? ''))) {
            throw new \DomainException('Loader paketi yüklenemedi.');
        }
        $size=(int)($file['size']??0);
        if($size<1||$size>Env::int('LOADER_UPLOAD_MAX_BYTES',2147483648))throw new \DomainException('Loader paketi boyutu izin verilen aralıkta değil.');
        $extension=strtolower(pathinfo((string)($file['name']??''),PATHINFO_EXTENSION));
        $mime=(new \finfo(FILEINFO_MIME_TYPE))->file((string)$file['tmp_name']);
        if(!in_array($extension,self::EXTENSIONS,true)||!in_array($mime,self::MIMES,true))throw new \DomainException('Loader paketi türü desteklenmiyor.');
        $directory=$this->storageDir();
        if(!is_dir($directory)&&!mkdir($directory,0750,true)&&!is_dir($directory))throw new \RuntimeException('Loader storage oluşturulamadı.');
        $storageName=bin2hex(random_bytes(24)).'.'.$extension;
        $target=$directory.DIRECTORY_SEPARATOR.$storageName;
        if(!move_uploaded_file((string)$file['tmp_name'],$target))throw new \RuntimeException('Loader paketi private storage alanına yazılamadı.');
        return ['storage_name'=>$storageName,'original_name'=>basename((string)$file['name']),'sha256'=>hash_file('sha256',$target),'size_bytes'=>filesize($target),'mime_type'=>$mime];
    }

    /** Storage GC ve orphan tarama için mutlak dizin yolu. */
    public function directory(): string
    {
        return $this->storageDir();
    }

    public function path(string $storageName): string
    {
        return $this->storageDir().DIRECTORY_SEPARATOR.basename($storageName);
    }

    private function storageDir(): string
    {
        $configured=Env::get('LOADER_STORAGE_PATH','storage/loader');
        return preg_match('~^[a-zA-Z]:[\\\\/]~',$configured)
            ? $configured
            : WEB_ROOT.DIRECTORY_SEPARATOR.str_replace(['/','\\'],DIRECTORY_SEPARATOR,$configured);
    }
}
