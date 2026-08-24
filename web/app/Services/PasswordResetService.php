<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use App\Core\Env;

final class PasswordResetService
{
    public function request(string $email): void
    {
        $email=mb_strtolower(trim($email));
        if(!filter_var($email,FILTER_VALIDATE_EMAIL))return;
        $pdo=Database::connection();$s=$pdo->prepare("SELECT id,email FROM users WHERE email=? AND status='active' LIMIT 1");$s->execute([$email]);$user=$s->fetch();
        if(!$user||!Env::bool('MAIL_ENABLED',false))return;
        $plain=bin2hex(random_bytes(32));$hash=hash('sha256',$plain);
        $pdo->prepare('DELETE FROM password_reset_tokens WHERE user_id=? OR expires_at<NOW()')->execute([$user['id']]);
        $pdo->prepare('INSERT INTO password_reset_tokens(user_id,token_hash,expires_at) VALUES(?,?,DATE_ADD(NOW(),INTERVAL 30 MINUTE))')->execute([$user['id'],$hash]);
        $url=rtrim(Env::get('APP_URL',''),'/').'/reset-password?token='.$plain;
        $headers=['Content-Type: text/plain; charset=UTF-8','From: '.Env::get('MAIL_FROM','no-reply@localhost')];
        $sent=@mail($user['email'],'Animus Türkçe Yama şifre sıfırlama',"Şifrenizi yenilemek için 30 dakika geçerli bağlantı:\n".$url."\n\nBu isteği siz yapmadıysanız mesajı yok sayın.",implode("\r\n",$headers));
        if(!$sent)$pdo->prepare('DELETE FROM password_reset_tokens WHERE token_hash=?')->execute([$hash]);
    }

    public function reset(string $token,string $password,string $confirmation): void
    {
        if(!preg_match('/^[a-f0-9]{64}$/',$token))throw new \DomainException('Sıfırlama bağlantısı geçersiz.');
        if($password!==$confirmation)throw new \DomainException('Şifreler eşleşmiyor.');
        if(strlen($password)<12||!preg_match('/[A-Z]/',$password)||!preg_match('/[a-z]/',$password)||!preg_match('/\d/',$password))throw new \DomainException('Şifre en az 12 karakter, büyük/küçük harf ve sayı içermelidir.');
        $pdo=Database::connection();$pdo->beginTransaction();
        try{$s=$pdo->prepare('SELECT id,user_id FROM password_reset_tokens WHERE token_hash=? AND used_at IS NULL AND expires_at>NOW() FOR UPDATE');$s->execute([hash('sha256',$token)]);$row=$s->fetch();if(!$row)throw new \DomainException('Sıfırlama bağlantısı geçersiz veya süresi dolmuş.');$pdo->prepare('UPDATE users SET password_hash=? WHERE id=?')->execute([password_hash($password,PASSWORD_DEFAULT),$row['user_id']]);$pdo->prepare('UPDATE password_reset_tokens SET used_at=NOW() WHERE id=?')->execute([$row['id']]);$pdo->prepare('DELETE FROM api_tokens WHERE user_id=?')->execute([$row['user_id']]);$pdo->commit();}catch(\Throwable $error){if($pdo->inTransaction())$pdo->rollBack();throw $error;}
    }
}

