<?php
declare(strict_types=1);
require __DIR__.'/../../web/app/bootstrap.php';

use App\Core\Database;

$base=rtrim(getenv('TEST_API_URL')?:'http://127.0.0.1:8080','/').'/api';$tests=0;
function check(bool $value,string $label):void{global $tests;$tests++;if(!$value)throw new RuntimeException('FAIL: '.$label);echo 'PASS: '.$label.PHP_EOL;}
function callApi(string $method,string $path,?array $body=null,?string $token=null):array{
    global $base;$headers=['Accept: application/json'];if($body!==null)$headers[]='Content-Type: application/json';if($token)$headers[]='Authorization: Bearer '.$token;
    $context=stream_context_create(['http'=>['method'=>$method,'header'=>implode("\r\n",$headers),'content'=>$body===null?'':json_encode($body,JSON_THROW_ON_ERROR),'ignore_errors'=>true,'timeout'=>10]]);
    $raw=file_get_contents($base.$path,false,$context);$status=0;foreach($http_response_header??[] as $header)if(preg_match('#^HTTP/\S+\s+(\d+)#',$header,$match))$status=(int)$match[1];
    return [$status,json_decode($raw?:'{}',true)?:[]];
}

$email='auth-test-'.bin2hex(random_bytes(6)).'@example.test';$password='StrongPassword42';$pdo=Database::connection();
foreach(['register','login'] as $bucket)$pdo->prepare('DELETE FROM rate_limits WHERE bucket_key=?')->execute([hash('sha256',$bucket.'|127.0.0.1')]);
try{
    [$status]=callApi('GET','/games');check($status===401,'protected catalog rejects missing token');
    [$status]=callApi('POST','/auth/register',['display_name'=>'Auth Test','email'=>'bad','password'=>$password]);check($status===422,'register rejects invalid email');
    [$status]=callApi('POST','/auth/register',['display_name'=>'Auth Test','email'=>$email,'password'=>'weak']);check($status===422,'register rejects weak password');
    [$status,$registered]=callApi('POST','/auth/register',['display_name'=>'Auth Test','email'=>$email,'password'=>$password]);check($status===201&&isset($registered['data']['token'],$registered['data']['user']),'register success returns user and token');check(!isset($registered['data']['user']['password_hash']),'register response excludes password hash');
    [$status]=callApi('POST','/auth/register',['display_name'=>'Auth Test','email'=>$email,'password'=>$password]);check($status===422,'duplicate email rejected');
    [$status]=callApi('POST','/auth/login',['email'=>$email,'password'=>'WrongPassword42']);check($status===422,'wrong password rejected');
    [$status,$login]=callApi('POST','/auth/login',['email'=>$email,'password'=>$password]);$token=(string)($login['data']['token']??'');check($status===200&&strlen($token)>=32,'login success returns access token');
    [$status,$me]=callApi('GET','/auth/me',null,$token);check($status===200&&($me['data']['email']??'')===$email,'token restore/me succeeds');
    [$status,$games]=callApi('GET','/games',null,$token);check($status===200&&($games['meta']['count']??0)===38,'protected catalog accepts valid token');
    $stmt=$pdo->prepare('SELECT password_hash FROM users WHERE email=?');$stmt->execute([$email]);$hash=(string)$stmt->fetchColumn();check($hash!==$password&&password_verify($password,$hash),'password stored only as a valid hash');
    [$status]=callApi('POST','/auth/logout',[], $token);check($status===200,'logout succeeds');
    [$status]=callApi('GET','/auth/me',null,$token);check($status===401,'revoked token is rejected');
    echo "PASS: {$tests} auth HTTP assertions.".PHP_EOL;
}finally{
    $stmt=$pdo->prepare('SELECT id FROM users WHERE email=?');$stmt->execute([$email]);$id=$stmt->fetchColumn();if($id)$pdo->prepare('DELETE FROM users WHERE id=?')->execute([$id]);foreach(['register','login'] as $bucket)$pdo->prepare('DELETE FROM rate_limits WHERE bucket_key=?')->execute([hash('sha256',$bucket.'|127.0.0.1')]);
}
