<?php
declare(strict_types=1);
require __DIR__.'/../../../../app/bootstrap.php';
\App\Core\Cors::handle();
use App\Core\Database;
use App\Core\Http;
use App\Core\RateLimiter;
if(($_SERVER['REQUEST_METHOD']??'GET')!=='POST')Http::error('Yalnız POST desteklenir.',405);
RateLimiter::enforce('register',5,3600);
$body=Http::body();$email=mb_strtolower(trim((string)($body['email']??'')));$name=trim((string)($body['display_name']??''));$password=(string)($body['password']??'');
if(!filter_var($email,FILTER_VALIDATE_EMAIL))Http::error('Geçerli e-posta gerekli.',422);
if(mb_strlen($name)<2||mb_strlen($name)>100)Http::error('Görünen ad 2-100 karakter olmalıdır.',422);
if(strlen($password)<12||!preg_match('/[A-Z]/',$password)||!preg_match('/[a-z]/',$password)||!preg_match('/\d/',$password))Http::error('Şifre en az 12 karakter, büyük/küçük harf ve sayı içermelidir.',422);
try{Database::connection()->prepare("INSERT INTO users(email,password_hash,display_name,role,release_channel,status) VALUES(?,?,?,'user','stable','active')")->execute([$email,password_hash($password,PASSWORD_DEFAULT),$name]);Http::json(['ok'=>true,'data'=>['message'=>'Hesap oluşturuldu. Loader üzerinden giriş yapabilirsiniz.']],201);}catch(PDOException $e){if($e->getCode()==='23000')Http::error('Bu e-posta zaten kayıtlı.',409);Http::error('Kayıt tamamlanamadı.',500);}
