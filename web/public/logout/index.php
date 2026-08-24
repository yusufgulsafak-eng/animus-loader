<?php
declare(strict_types=1);
require __DIR__.'/../../app/bootstrap.php';
use App\Core\Csrf;
use App\Services\AuthService;
if(($_SERVER['REQUEST_METHOD']??'GET')==='POST'&&Csrf::verify($_POST['_csrf']??null))(new AuthService())->logout();
header('Location: /login/');exit;

