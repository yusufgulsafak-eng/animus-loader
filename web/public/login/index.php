<?php
declare(strict_types=1);
require __DIR__.'/../../app/bootstrap.php';
use App\Core\Csrf;
use App\Core\Session;
use App\Services\AuthService;
if(Session::user()){header('Location: /account/');exit;}
$error=null;
if(($_SERVER['REQUEST_METHOD']??'GET')==='POST'){
    if(!Csrf::verify($_POST['_csrf']??null))$error='Güvenlik doğrulaması başarısız.';
    else try{(new AuthService())->login($_POST['email']??'',$_POST['password']??'');header('Location: /account/');exit;}catch(Throwable $e){$error=$e->getMessage();}
}
?><!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Hesabım</title><link rel="stylesheet" href="/assets/app.css"></head><body class="login-page"><main class="login-card"><div class="brand large"><span class="brand-mark">A</span><span>KULLANICI GİRİŞİ</span></div><p class="muted">Oyun, yama ve abonelik durumunu görüntüleyin.</p><?php if($error):?><div class="alert error"><?=htmlspecialchars($error)?></div><?php endif?><form method="post"><input type="hidden" name="_csrf" value="<?=htmlspecialchars(Csrf::token())?>"><label>E-posta<input name="email" type="email" required></label><label>Şifre<input name="password" type="password" required></label><button class="button primary wide">Giriş Yap</button></form><a href="/register/">Hesabınız yok mu? Kayıt olun.</a></main></body></html>

