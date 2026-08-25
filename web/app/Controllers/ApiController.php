<?php
declare(strict_types=1);

namespace App\Controllers;

use App\Core\Database;
use App\Core\Env;
use App\Core\Http;
use App\Core\RateLimiter;
use App\Services\AuthService;
use App\Services\AdminService;
use App\Services\CatalogService;
use App\Services\ManifestService;
use App\Services\ManifestValidator;
use App\Services\PatchStorage;
use App\Support\AdminActions;

final class ApiController
{
    private AuthService $auth;
    public function __construct(){ $this->auth=new AuthService(); }

    public function handle(string $method,string $path): never
    {
        $path=preg_replace('#^/api/v1(?=/|$)#','/api',$path);
        try {
            if($method==='POST'&&$path==='/api/auth/login')$this->login();
            if($method==='POST'&&$path==='/api/auth/register')$this->register();
            if($method==='POST'&&$path==='/api/auth/logout')$this->logout();
            if($method==='GET'&&$path==='/api/auth/me')$this->me();
            if($method==='GET'&&$path==='/api/user/me')$this->me();
            if($method==='GET'&&$path==='/api/games')$this->games();
            if($method==='GET'&&$path==='/api/admin/panel')$this->adminPanel();
            if($method==='POST'&&$path==='/api/admin/action')$this->adminAction();
            if($method==='GET'&&preg_match('#^/api/games/(\d+)$#',$path,$m))$this->game((int)$m[1]);
            if($method==='GET'&&preg_match('#^/api/games/(\d+)/patch$#',$path,$m))$this->gamePatch((int)$m[1]);
            if($method==='GET'&&preg_match('#^/api/patches/(\d+)/manifest$#',$path,$m))$this->manifest((int)$m[1]);
            if($method==='POST'&&preg_match('#^/api/patches/(\d+)/download-token$#',$path,$m))$this->downloadToken((int)$m[1]);
            if($method==='GET'&&preg_match('#^/api/download/([a-f0-9]{64})$#',$path,$m))$this->download($m[1]);
            if($method==='GET'&&$path==='/api/loader/config')$this->loaderConfig();
            if($method==='GET'&&$path==='/api/loader/latest')$this->loaderLatest();
            if($method==='GET'&&preg_match('#^/api/loader/download/([0-9A-Za-z.+-]+)$#',$path,$m))$this->loaderDownload($m[1]);
            Http::error('Endpoint bulunamadı.',404);
        } catch(\DomainException $e){Http::error($e->getMessage(),422);} catch(\RuntimeException $e){
            if($e->getMessage()==='AUTH_REQUIRED')Http::error('Giriş gerekli.',401);
            if($e->getMessage()==='ADMIN_REQUIRED')Http::error('Yetkiniz yok.',403);
            Http::error(Env::bool('APP_DEBUG')?$e->getMessage():'İşlem tamamlanamadı.',500);
        } catch(\Throwable $e){Http::error(Env::bool('APP_DEBUG')?$e->getMessage():'Sunucu hatası.',500);}
    }

    private function login(): never
    {
        RateLimiter::enforce('login',10,60);$b=Http::body();$user=$this->auth->login((string)($b['email']??''),(string)($b['password']??''));$token=$this->auth->issueApiToken((int)$user['id']);Http::json(['ok'=>true,'data'=>['user'=>$user,'token'=>$token]]);
    }
    private function register(): never
    {
        RateLimiter::enforce('register',5,3600);
        $body=Http::body();$this->auth->register((string)($body['email']??''),(string)($body['display_name']??''),(string)($body['password']??''));$user=$this->auth->login((string)($body['email']??''),(string)($body['password']??''));$token=$this->auth->issueApiToken((int)$user['id']);Http::json(['ok'=>true,'data'=>['message'=>'Hesap oluşturuldu.','user'=>$user,'token'=>$token]],201);
    }
    private function logout():never{$this->auth->requireUser();$this->auth->logout();Http::json(['ok'=>true,'data'=>null]);}
    private function me():never{Http::json(['ok'=>true,'data'=>$this->auth->requireUser()]);}
    private function adminPanel():never{$this->auth->requireAdmin();Http::json(['ok'=>true,'data'=>(new AdminService())->panelData()]);}
    private function adminAction():never
    {
        RateLimiter::enforce('admin-action',180,60);
        $user=$this->auth->requireAdmin();
        $body=Http::body();
        $result=AdminActions::dispatch((string)($body['action']??''),$body,$_FILES,$user);
        Http::json(['ok'=>true,'data'=>$result]);
    }
    private function games():never{$u=$this->auth->requireUser();$games=(new CatalogService())->games($u,['q'=>trim((string)($_GET['q']??'')),'access'=>$_GET['access']??null]);Http::json(['ok'=>true,'data'=>$games,'meta'=>['count'=>count($games)]]);}
    private function game(int$id):never{$u=$this->auth->requireUser();$g=(new CatalogService())->game($id,$u);if(!$g)Http::error('Oyun bulunamadı.',404);Http::json(['ok'=>true,'data'=>$g]);}
    private function gamePatch(int$id):never{$u=$this->auth->requireUser();$p=(new CatalogService())->activePatch($id,$u);if(!$p)Http::error('Yayınlanmış patch bulunamadı.',404);if($p['access_type']==='premium'&&!$this->auth->canAccessPremium($u))Http::error('Premium abonelik gerekli.',403);Http::json(['ok'=>true,'data'=>$p]);}
    private function manifest(int$id):never{$u=$this->auth->requireUser();$this->assertManifestAccess($id,$u);$m=(new ManifestService())->build($id);Http::json(['ok'=>true,'data'=>$m]);}

    private function downloadToken(int$id):never
    {
        RateLimiter::enforce('download-token',30,60);$u=$this->auth->requireUser();$this->assertManifestAccess($id,$u);$pdo=Database::connection();$s=$pdo->prepare('SELECT id,source_type,external_url FROM patch_archives WHERE patch_version_id=?');$s->execute([$id]);$archive=$s->fetch();if(!$archive)Http::error('Patch arşivi bulunamadı.',404);$ttl=Env::int('DOWNLOAD_TOKEN_TTL',300);if(($archive['source_type']??'server')==='external'){if(empty($archive['external_url']))Http::error('Harici patch URL tanımlı değil.',500);Http::json(['ok'=>true,'data'=>['url'=>$archive['external_url'],'expires_in'=>$ttl,'source_type'=>'external']]);}$plain=bin2hex(random_bytes(32));$pdo->prepare('INSERT INTO download_tokens(user_id,patch_archive_id,token_hash,expires_at) VALUES(?,?,?,DATE_ADD(NOW(),INTERVAL ? SECOND))')->execute([$u['id'],$archive['id'],hash('sha256',$plain),$ttl]);Http::json(['ok'=>true,'data'=>['url'=>rtrim(Env::get('APP_URL',''),'/').'/api/download/'.$plain,'expires_in'=>$ttl,'source_type'=>'server']]);
    }

    private function download(string$token):never
    {
        $pdo=Database::connection();$pdo->beginTransaction();try{$s=$pdo->prepare('SELECT dt.id,dt.user_id,dt.patch_archive_id,pa.storage_name,pa.original_name,pa.size_bytes,pa.mime_type,pa.source_type,pa.external_url FROM download_tokens dt JOIN patch_archives pa ON pa.id=dt.patch_archive_id WHERE dt.token_hash=? AND dt.expires_at>NOW() AND dt.used_at IS NULL FOR UPDATE');$s->execute([hash('sha256',$token)]);$row=$s->fetch();if(!$row){$pdo->rollBack();Http::error('İndirme tokenı geçersiz veya süresi dolmuş.',410);}$pdo->prepare('UPDATE download_tokens SET used_at=NOW() WHERE id=?')->execute([$row['id']]);$ip=hash('sha256',($_SERVER['REMOTE_ADDR']??'unknown').'|'.Env::get('APP_KEY','local'));$pdo->prepare("INSERT INTO download_logs(user_id,patch_archive_id,ip_hash,user_agent,status) VALUES(?,?,?,?, 'started')")->execute([$row['user_id'],$row['patch_archive_id'],$ip,substr($_SERVER['HTTP_USER_AGENT']??'',0,500)]);$log=(int)$pdo->lastInsertId();$pdo->commit();if(($row['source_type']??'server')==='external'&&!empty($row['external_url'])){$pdo->prepare("UPDATE download_logs SET status='completed',bytes_sent=? WHERE id=?")->execute([$row['size_bytes'],$log]);header('Location: '.$row['external_url'],true,302);exit;}$path=(new PatchStorage())->path($row['storage_name']);if(!is_file($path)){$pdo->prepare("UPDATE download_logs SET status='failed' WHERE id=?")->execute([$log]);throw new \RuntimeException('Arşiv storage içinde yok.');}header('Content-Type: '.$row['mime_type']);header('Content-Length: '.$row['size_bytes']);header('Content-Disposition: attachment; filename="'.str_replace('"','',$row['original_name']).'"');header('X-Content-Type-Options: nosniff');readfile($path);$pdo->prepare("UPDATE download_logs SET status='completed',bytes_sent=? WHERE id=?")->execute([$row['size_bytes'],$log]);exit;}catch(\Throwable$e){if($pdo->inTransaction())$pdo->rollBack();throw$e;}
    }

    private function loaderConfig():never
    {
        $row=Database::connection()->query('SELECT * FROM loader_config WHERE id=1')->fetch()?:['app_name'=>'Türkçe Yama Loader','accent_color'=>'#B7F34A','library_title'=>'Kütüphane'];
        $row['announcements']=Database::connection()->query("SELECT id,title,body,starts_at,ends_at FROM announcements WHERE is_active=1 AND (starts_at IS NULL OR starts_at<=NOW()) AND (ends_at IS NULL OR ends_at>NOW()) ORDER BY created_at DESC LIMIT 10")->fetchAll();
        $row['banners']=Database::connection()->query('SELECT id,title,image_path,target_url FROM banners WHERE is_active=1 ORDER BY sort_order,id LIMIT 10')->fetchAll();
        $row['branding']=['login_background'=>$this->brandingConfig($row,'login'),'library_background'=>$this->brandingConfig($row,'library')];
        Http::json(['ok'=>true,'data'=>$row]);
    }
    private function brandingConfig(array$row,string$slot):array{$prefix=$slot.'_background_';$type=in_array($row[$prefix.'type']??'', ['default','image','video'],true)?$row[$prefix.'type']:'default';$image=$row[$prefix.'image']??null;$video=$row[$prefix.'video']??null;$fallback=$row[$prefix.'fallback']??null;if($slot==='login'&&$type==='default'&&!$image&&!$video&&!empty($row['login_background_url'])){$type='image';$image=$row['login_background_url'];}$overlay=max(0,min(100,(int)($row[$prefix.'overlay']??($slot==='login'?60:55))));return ['type'=>$type,'image_url'=>$image,'video_url'=>$video,'fallback_url'=>$fallback,'overlay'=>$overlay,'version'=>substr(hash('sha256',json_encode([$type,$image,$video,$fallback,$row['updated_at']??''])),0,16)];}
    private function loaderLatest():never{$channel=in_array($_GET['channel']??'', ['stable','beta','internal'],true)?$_GET['channel']:'stable';$s=Database::connection()->prepare('SELECT version,sha256,size_bytes,mandatory,release_notes,published_at FROM loader_versions WHERE channel=? ORDER BY published_at DESC,id DESC LIMIT 1');$s->execute([$channel]);$v=$s->fetch();if(!$v)Http::json(['ok'=>true,'data'=>null]);$v['download_url']=rtrim(Env::get('APP_URL',''),'/').'/api/loader/download/'.$v['version'];Http::json(['ok'=>true,'data'=>$v]);}
    private function loaderDownload(string$version):never{$s=Database::connection()->prepare("SELECT storage_name,size_bytes FROM loader_versions WHERE version=? ORDER BY FIELD(channel,'stable','beta','internal'),id DESC LIMIT 1");$s->execute([$version]);$row=$s->fetch();if(!$row)Http::error('Loader sürümü bulunamadı.',404);$path=(new \App\Services\LoaderStorage())->path($row['storage_name']);if(!is_file($path))Http::error('Loader paketi storage içinde bulunamadı.',404);header('Content-Type: application/octet-stream');header('Content-Length: '.$row['size_bytes']);header('Content-Disposition: attachment; filename="Animus-Turkce-Yama-'.$version.'.bin"');header('X-Content-Type-Options: nosniff');readfile($path);exit;}
    private function assertManifestAccess(int$id,array$u):void{$s=Database::connection()->prepare("SELECT pv.access_type,pv.status,pv.channel FROM patch_versions pv WHERE pv.id=?");$s->execute([$id]);$p=$s->fetch();if(!$p)Http::error('Patch bulunamadı.',404);if($p['status']!=='PUBLISHED')Http::error('Patch yayınlanmamış.',409);$allowed=['stable'=>['stable'],'beta'=>['stable','beta'],'internal'=>['stable','beta','internal']][$u['release_channel']??'stable'];if(!in_array($p['channel'],$allowed,true))Http::error('Release kanalına erişiminiz yok.',403);if($p['access_type']==='premium'&&!$this->auth->canAccessPremium($u))Http::error('Premium abonelik gerekli.',403);}
}