<?php
declare(strict_types=1);

namespace App\Controllers;

use App\Core\Csrf;
use App\Core\Env;
use App\Core\Http;
use App\Core\Session;
use App\Core\View;
use App\Services\AdminService;
use App\Services\AuthService;
use App\Services\ManifestService;

final class AdminController
{
    public function handle(string$method,string$path):never
    {
        $auth=new AuthService();
        if($path==='/admin/login'&&$method==='GET')View::render('admin-login',['csrf'=>Csrf::token()]);
        if($path==='/admin/login'&&$method==='POST'){$this->csrf($_POST['_csrf']??null);try{$u=$auth->login($_POST['email']??'',$_POST['password']??'');if(!in_array($u['role'],['admin','super_admin'],true))throw new \DomainException('Admin yetkisi gerekli.');header('Location: /admin');exit;}catch(\Throwable$e){View::render('admin-login',['csrf'=>Csrf::token(),'error'=>$e->getMessage()]);}}
        if(!Session::isAdmin()){header('Location: /admin/login');exit;}
        if($path==='/admin/logout'&&$method==='POST'){$this->csrf($_POST['_csrf']??null);$auth->logout();header('Location: /admin/login');exit;}
        if($path==='/admin/action'&&$method==='POST')$this->action();
        View::render('admin',['csrf'=>Csrf::token(),'user'=>Session::user(),'data'=>(new AdminService())->panelData()]);
    }

    private function action():never
    {
        $this->csrf($_SERVER['HTTP_X_CSRF_TOKEN']??($_POST['_csrf']??null));$body=Http::body();$action=$body['action']??'';$admin=new AdminService();$uid=(int)Session::user()['id'];
        try{$result=match($action){
            'save_game'=>['id'=>$admin->saveGame($body['game']??[],$uid)],
            'duplicate_game'=>['id'=>$admin->duplicateGame((int)($body['game_id']??0),$uid)],
            'set_game_status'=>(function()use($admin,$body,$uid){$admin->setGameStatus((int)($body['game_id']??0),(bool)($body['active']??false),$uid);return null;})(),
            'upload_game_image'=>['path'=>$admin->saveGameImage((int)($_POST['game_id']??0),(string)($_POST['kind']??''),$_FILES['image']??[],$uid)],
            'delete_game_image'=>(function()use($admin,$body,$uid){$admin->deleteGameImage((int)($body['game_id']??0),(string)($body['kind']??''),$uid);return null;})(),
            'create_patch'=>['id'=>$admin->createPatchVersion($_POST,$_FILES['archive']??[],$uid)],
            'load_patch_builder'=>$admin->builderData((int)($body['version_id']??0)),
            'save_actions'=>(function()use($admin,$body,$uid){$admin->saveActions((int)$body['version_id'],$body['actions']??[],$uid);return null;})(),
            'test_manifest'=>(function()use($body){$m=(new ManifestService())->build((int)$body['version_id']);return ['manifest'=>$m,'errors'=>(new \App\Services\ManifestValidator())->validate($m)];})(),
            'publish_patch'=>(function()use($admin,$body,$uid){$admin->publish((int)$body['version_id'],$uid);return null;})(),
            'rollback_patch'=>(function()use($admin,$body,$uid){$admin->rollbackPatch((int)$body['version_id'],$uid);return null;})(),
            'set_patch_status'=>(function()use($admin,$body,$uid){$admin->setPatchStatus((int)($body['version_id']??0),(string)($body['status']??''),$uid);return null;})(),
            'save_category'=>['id'=>$admin->saveCategory($body['category']??[],$uid)],
            'save_announcement'=>['id'=>$admin->saveAnnouncement($body['announcement']??[],$uid)],
            'delete_announcement'=>(function()use($admin,$body,$uid){$admin->deleteAnnouncement((int)($body['id']??0),$uid);return null;})(),
            'save_banner'=>['id'=>$admin->saveBanner($_POST,$_FILES['image']??[],$uid)],
            'delete_banner'=>(function()use($admin,$body,$uid){$admin->deleteBanner((int)($body['id']??0),$uid);return null;})(),
            'save_subscription'=>['id'=>$admin->saveSubscription($body['subscription']??[],$uid)],
            'set_subscription_status'=>(function()use($admin,$body,$uid){$admin->setSubscriptionStatus((int)($body['id']??0),(string)($body['status']??''),$uid);return null;})(),
            'update_user'=>(function()use($admin,$body,$uid){$admin->updateUser($body['user']??[],$uid);return null;})(),
            'create_loader_version'=>['id'=>$admin->createLoaderVersion($_POST,$_FILES['package']??[],$uid)],
            'save_branding_media'=>$admin->saveBrandingMedia($_POST,$_FILES,$uid),
            'reset_branding_media'=>$admin->resetBrandingMedia((string)($body['slot']??''),$uid),
            'save_loader_config'=>(function()use($admin,$body,$uid){$admin->saveLoaderConfig($body['config']??[],$uid);return null;})(),
            default=>throw new \DomainException('Bilinmeyen admin işlemi.'),};Http::json(['ok'=>true,'data'=>$result]);}catch(\DomainException$e){Http::error($e->getMessage(),422);}catch(\Throwable$e){error_log('Admin action error ['.$action.']: '.$e->getMessage());Http::error(Env::bool('APP_DEBUG')?'İşlem tamamlanamadı: '.$e->getMessage():'Sunucu tarafında bir hata oluştu.',500);}
    }
    private function csrf(?string$t):void{if(!Csrf::verify($t))Http::error('CSRF doğrulaması başarısız.',419);}
}
