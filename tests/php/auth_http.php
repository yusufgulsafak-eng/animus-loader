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

$email='auth-test-'.bin2hex(random_bytes(6)).'@example.test';$password='StrongPassword42';$pdo=Database::connection();$gameId=0;$categoryId=0;$announcementId=0;
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
    [$status]=callApi('GET','/admin/panel',null,$token);check($status===403,'normal user cannot access loader admin API');
    $pdo->prepare("UPDATE users SET role='admin' WHERE email=?")->execute([$email]);
    [$status,$adminPanel]=callApi('GET','/admin/panel',null,$token);check($status===200&&isset($adminPanel['data']['games'],$adminPanel['data']['versions']),'admin role can access loader admin API');
    $pdo->prepare("UPDATE users SET role='super_admin' WHERE email=?")->execute([$email]);
    [$status]=callApi('GET','/admin/panel',null,$token);check($status===200,'super_admin role can access loader admin API');
    $suffix=bin2hex(random_bytes(4));
    [$status,$category]=callApi('POST','/admin/action',['action'=>'save_category','category'=>['name'=>'API Test','slug'=>'api-test-'.$suffix,'sort_order'=>999,'is_active'=>true]],$token);$categoryId=(int)($category['data']['id']??0);check($status===200&&$categoryId>0,'admin API creates category');
    [$status,$game]=callApi('POST','/admin/action',['action'=>'save_game','game'=>['name'=>'API Test Game','slug'=>'api-test-game-'.$suffix,'executable'=>'Game.exe','process_name'=>'Game.exe','access_type'=>'free','translation_percent'=>1,'minimum_loader_version'=>'0.1.0','supported_stores'=>['manual'],'required_files'=>['Game.exe'],'optional_files'=>[],'category_ids'=>[$categoryId],'is_active'=>true]],$token);$gameId=(int)($game['data']['id']??0);check($status===200&&$gameId>0,'admin API creates game');
    [$status]=callApi('POST','/admin/action',['action'=>'save_game','game'=>['id'=>$gameId,'name'=>'API Test Game Updated','slug'=>'api-test-game-'.$suffix,'executable'=>'Game.exe','process_name'=>'Game.exe','access_type'=>'free','translation_percent'=>2,'minimum_loader_version'=>'0.1.0','supported_stores'=>['manual'],'required_files'=>['Game.exe'],'optional_files'=>[],'category_ids'=>[$categoryId],'is_active'=>true]],$token);check($status===200,'admin API updates game');
    [$status,$announcement]=callApi('POST','/admin/action',['action'=>'save_announcement','announcement'=>['title'=>'API Test','body'=>'Temporary test announcement','audience'=>'admin','is_active'=>true]],$token);$announcementId=(int)($announcement['data']['id']??0);check($status===200&&$announcementId>0,'admin API creates announcement');
    [$status]=callApi('POST','/admin/action',['action'=>'delete_announcement','id'=>$announcementId],$token);check($status===200,'admin API deletes announcement');$announcementId=0;
    [$status]=callApi('POST','/admin/action',['action'=>'delete_game','game_id'=>$gameId],$token);check($status===200,'admin API deletes game');$gameId=0;
    [$status]=callApi('POST','/admin/action',['action'=>'delete_category','id'=>$categoryId],$token);check($status===200,'admin API deletes category');$categoryId=0;
    $stmt=$pdo->prepare('SELECT password_hash FROM users WHERE email=?');$stmt->execute([$email]);$hash=(string)$stmt->fetchColumn();check($hash!==$password&&password_verify($password,$hash),'password stored only as a valid hash');
    [$status]=callApi('POST','/auth/logout',[], $token);check($status===200,'logout succeeds');
    [$status]=callApi('GET','/auth/me',null,$token);check($status===401,'revoked token is rejected');
    echo "PASS: {$tests} auth HTTP assertions.".PHP_EOL;
}finally{
    if($announcementId)$pdo->prepare('DELETE FROM announcements WHERE id=?')->execute([$announcementId]);
    if($gameId)$pdo->prepare('DELETE FROM games WHERE id=?')->execute([$gameId]);
    if($categoryId)$pdo->prepare('DELETE FROM categories WHERE id=?')->execute([$categoryId]);
    $stmt=$pdo->prepare('SELECT id FROM users WHERE email=?');$stmt->execute([$email]);$id=$stmt->fetchColumn();if($id)$pdo->prepare('DELETE FROM users WHERE id=?')->execute([$id]);foreach(['register','login'] as $bucket)$pdo->prepare('DELETE FROM rate_limits WHERE bucket_key=?')->execute([hash('sha256',$bucket.'|127.0.0.1')]);
}
