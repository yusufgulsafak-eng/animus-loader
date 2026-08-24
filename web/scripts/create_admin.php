<?php
declare(strict_types=1);
require __DIR__ . '/../app/bootstrap.php';
use App\Core\Database;
if(PHP_SAPI!=='cli'){fwrite(STDERR,"Bu araç yalnız CLI üzerinden çalıştırılır.\n");exit(1);}
function ask(string $label):string{fwrite(STDOUT,$label.': ');return trim((string)fgets(STDIN));}
$email=mb_strtolower(ask('Admin e-posta'));
$name=ask('Görünen ad');
$password=ask('En az 12 karakterli güçlü şifre');
$confirm=ask('Şifre tekrar');
if(!filter_var($email,FILTER_VALIDATE_EMAIL)){fwrite(STDERR,"Geçerli e-posta girilmedi.\n");exit(2);}
if(mb_strlen($name)<2){fwrite(STDERR,"Görünen ad çok kısa.\n");exit(2);}
if(strlen($password)<12||!preg_match('/[A-Z]/',$password)||!preg_match('/[a-z]/',$password)||!preg_match('/\d/',$password)){fwrite(STDERR,"Şifre en az 12 karakter, büyük/küçük harf ve sayı içermelidir.\n");exit(2);}
if(!hash_equals($password,$confirm)){fwrite(STDERR,"Şifreler eşleşmiyor.\n");exit(2);}
$pdo=Database::connection();$stmt=$pdo->prepare("INSERT INTO users(email,password_hash,display_name,role,release_channel,status,email_verified_at) VALUES(?,?,?,'super_admin','internal','active',NOW())");
try{$stmt->execute([$email,password_hash($password,PASSWORD_DEFAULT),$name]);echo "Super admin güvenli şekilde oluşturuldu.\n";}catch(Throwable $e){fwrite(STDERR,"Admin oluşturulamadı; e-posta daha önce kullanılmış olabilir.\n");exit(3);}

