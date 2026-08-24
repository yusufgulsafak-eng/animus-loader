<?php
declare(strict_types=1);

require __DIR__ . '/../app/bootstrap.php';

use App\Controllers\AdminController;
use App\Controllers\ApiController;
use App\Core\View;
use App\Core\Csrf;
use App\Services\AuthService;
use App\Services\PasswordResetService;

$path=parse_url($_SERVER['REQUEST_URI']??'/',PHP_URL_PATH)?:'/';
$method=strtoupper($_SERVER['REQUEST_METHOD']??'GET');

\App\Core\Cors::handle();
header_remove('X-Powered-By');
header('Referrer-Policy: strict-origin-when-cross-origin');
header('X-Frame-Options: DENY');
header('X-Content-Type-Options: nosniff');
$forwardedProto=strtolower(trim(explode(',',$_SERVER['HTTP_X_FORWARDED_PROTO']??'')[0]??''));
$isHttps=(!empty($_SERVER['HTTPS'])&&strtolower((string)$_SERVER['HTTPS'])!=='off')||$forwardedProto==='https';
if($isHttps)header('Strict-Transport-Security: max-age=31536000; includeSubDomains');
header("Permissions-Policy: camera=(), microphone=(), geolocation=()");

if($method==='GET'&&preg_match('#^/media/branding/([a-f0-9]{48}\.(?:jpe?g|png|webp|mp4|webm))$#i',$path,$media))(new \App\Services\BrandingMediaStorage())->stream($media[1]);
if(str_starts_with($path,'/api/'))(new ApiController())->handle($method,rtrim($path,'/')?:'/');
if(str_starts_with($path,'/admin'))(new AdminController())->handle($method,rtrim($path,'/')?:'/admin');
$page=rtrim($path,'/')?:'/';
$esc=static fn($value)=>htmlspecialchars((string)$value,ENT_QUOTES,'UTF-8');
if($page==='/register'){if($method==='GET')View::render('register',['csrf'=>Csrf::token(),'esc'=>$esc]);if($method==='POST'){if(!Csrf::verify($_POST['_csrf']??null))\App\Core\Http::error('CSRF doğrulaması başarısız.',419);try{(new AuthService())->register((string)($_POST['email']??''),(string)($_POST['display_name']??''),(string)($_POST['password']??''));View::render('register',['csrf'=>Csrf::token(),'esc'=>$esc,'message'=>'Hesap oluşturuldu. Loader üzerinden giriş yapabilirsiniz.']);}catch(\Throwable$error){View::render('register',['csrf'=>Csrf::token(),'esc'=>$esc,'error'=>$error instanceof \DomainException?$error->getMessage():'Kayıt tamamlanamadı.']);}}}
if($page==='/forgot-password'){if($method==='GET')View::render('forgot-password',['csrf'=>Csrf::token(),'esc'=>$esc]);if($method==='POST'){if(!Csrf::verify($_POST['_csrf']??null))\App\Core\Http::error('CSRF doğrulaması başarısız.',419);\App\Core\RateLimiter::enforce('password-reset',5,3600);(new PasswordResetService())->request((string)($_POST['email']??''));View::render('forgot-password',['csrf'=>Csrf::token(),'esc'=>$esc,'message'=>'Hesap mevcutsa sıfırlama bağlantısı gönderildi.']);}}
if($page==='/reset-password'){if($method==='GET')View::render('reset-password',['csrf'=>Csrf::token(),'esc'=>$esc,'token'=>(string)($_GET['token']??'')]);if($method==='POST'){if(!Csrf::verify($_POST['_csrf']??null))\App\Core\Http::error('CSRF doğrulaması başarısız.',419);try{(new PasswordResetService())->reset((string)($_POST['token']??''),(string)($_POST['password']??''),(string)($_POST['confirmation']??''));View::render('reset-password',['csrf'=>Csrf::token(),'esc'=>$esc,'token'=>'','message'=>'Şifreniz yenilendi. Açık loader oturumları kapatıldı.']);}catch(\Throwable$error){View::render('reset-password',['csrf'=>Csrf::token(),'esc'=>$esc,'token'=>(string)($_POST['token']??''),'error'=>$error instanceof \DomainException?$error->getMessage():'Şifre yenilenemedi.']);}}}
if($path==='/health'){header('Content-Type: application/json');echo json_encode(['ok'=>true,'time'=>gmdate(DATE_ATOM)]);exit;}
View::render('home');
