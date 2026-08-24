<?php
declare(strict_types=1);
require __DIR__.'/../../app/bootstrap.php';
use App\Core\Csrf;
use App\Core\Database;
use App\Core\Session;
use App\Services\AuthService;
use App\Services\CatalogService;
$user=Session::user();if(!$user){header('Location: /login/');exit;}
$user['premium']=(new AuthService())->canAccessPremium($user);
$games=(new CatalogService())->games($user);
$stmt=Database::connection()->prepare("SELECT plan_name,status,starts_at,ends_at FROM subscriptions WHERE user_id=? ORDER BY created_at DESC LIMIT 1");$stmt->execute([$user['id']]);$subscription=$stmt->fetch();
$esc=static fn($v)=>htmlspecialchars((string)$v,ENT_QUOTES,'UTF-8');
?><!doctype html><html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Kütüphanem</title><link rel="stylesheet" href="/assets/app.css"><style>.account{padding:35px 6vw}.account-head{display:flex;justify-content:space-between;align-items:center}.account-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:18px;margin-top:25px}.account-game{min-height:300px;background-size:cover!important;display:flex;align-items:flex-end;position:relative;overflow:hidden}.account-game:after{content:"";position:absolute;inset:30% 0 0;background:linear-gradient(transparent,#09070e)}.account-game div{z-index:1}.account-game h3{font-size:21px;margin:10px 0}</style></head><body><nav class="topbar"><a class="brand" href="/"><span class="brand-mark">A</span><span>ANIMUS PATCH</span></a><form method="post" action="/logout/"><input type="hidden" name="_csrf" value="<?=$esc(Csrf::token())?>"><button class="button ghost">Çıkış</button></form></nav><main class="account"><header class="account-head"><div><span class="eyebrow">HOŞ GELDİN</span><h1><?=$esc($user['display_name'])?></h1><p class="muted"><?=$esc($user['email'])?> · <?=$user['premium']?'Premium erişim':'Ücretsiz erişim'?></p></div><div class="content-card"><small>Abonelik</small><h3><?=$esc($subscription['plan_name']??'Ücretsiz')?></h3><span class="tag <?=$user['premium']?'lime':''?>"><?=$esc($subscription['status']??'active')?></span></div></header><h2>Yama Kütüphanesi</h2><section class="account-grid"><?php foreach($games as $game):?><article class="content-card account-game" style="background:url('<?=$esc($game['cover_path'])?>') center/cover"><div><span class="tag <?=$game['access_type']==='free'||$user['premium']?'lime':''?>"><?=$esc($game['access_type'])?></span><h3><?=$esc($game['name'])?></h3><small><?=$game['patch_version']?'Yama '.$esc($game['patch_version']):'Yama hazırlanıyor'?> · %<?=(int)$game['translation_percent']?></small></div></article><?php endforeach?></section></main></body></html>
