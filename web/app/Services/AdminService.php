<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use App\Core\PathGuard;
use PDO;
use DomainException;
use Throwable;

final class AdminService
{
    public function dashboard(): array
    {
        $pdo=Database::connection();
        return [
            'games'=>(int)$pdo->query('SELECT COUNT(*) FROM games')->fetchColumn(),
            'active_games'=>(int)$pdo->query('SELECT COUNT(*) FROM games WHERE is_active=1')->fetchColumn(),
            'patches'=>(int)$pdo->query('SELECT COUNT(*) FROM patch_versions')->fetchColumn(),
            'stable'=>(int)$pdo->query("SELECT COUNT(*) FROM patch_versions WHERE status='PUBLISHED' AND channel='stable'")->fetchColumn(),
            'beta'=>(int)$pdo->query("SELECT COUNT(*) FROM patch_versions WHERE status='PUBLISHED' AND channel='beta'")->fetchColumn(),
            'users'=>(int)$pdo->query('SELECT COUNT(*) FROM users')->fetchColumn(),
            'today_downloads'=>(int)$pdo->query('SELECT COUNT(*) FROM download_logs WHERE created_at>=CURRENT_DATE')->fetchColumn(),
            'downloads'=>(int)$pdo->query("SELECT COUNT(*) FROM download_logs WHERE status='completed'")->fetchColumn(),
        ];
    }

    public function panelData(): array
    {
        $pdo=Database::connection();
        $games=$pdo->query('SELECT g.*,GROUP_CONCAT(c.name SEPARATOR ", ") categories,GROUP_CONCAT(c.id) category_ids_csv FROM games g LEFT JOIN game_categories gc ON gc.game_id=g.id LEFT JOIN categories c ON c.id=gc.category_id GROUP BY g.id ORDER BY g.updated_at DESC LIMIT 500')->fetchAll();
        $rules=$pdo->query("SELECT game_id,rule_type,rule_value FROM game_detection_rules WHERE rule_type IN ('required_file','optional_file') ORDER BY sort_order,id")->fetchAll();
        $rulesByGame=[];foreach($rules as $rule)$rulesByGame[(int)$rule['game_id']][$rule['rule_type']==='required_file'?'required_files':'optional_files'][]=$rule['rule_value'];
        foreach($games as &$game){$game['supported_stores']=json_decode($game['supported_stores']??'[]',true)?:['manual'];$game['required_files']=$rulesByGame[(int)$game['id']]['required_files']??[];$game['optional_files']=$rulesByGame[(int)$game['id']]['optional_files']??[];$game['category_ids']=array_values(array_filter(array_map('intval',explode(',',(string)($game['category_ids_csv']??'')))));unset($game['category_ids_csv']);}unset($game);
        return [
            'stats'=>$this->dashboard(),
            'games'=>$games,
            'categories'=>$pdo->query('SELECT * FROM categories ORDER BY sort_order,name')->fetchAll(),
            'versions'=>$pdo->query('SELECT pv.*,g.name game_name,pa.original_name,pa.size_bytes,pa.source_type,pa.external_url FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id LEFT JOIN patch_archives pa ON pa.patch_version_id=pv.id ORDER BY pv.created_at DESC LIMIT 500')->fetchAll(),
            'users'=>$pdo->query('SELECT id,email,display_name,role,release_channel,status,created_at FROM users ORDER BY created_at DESC LIMIT 500')->fetchAll(),
            'subscriptions'=>$pdo->query('SELECT s.*,u.email,u.display_name FROM subscriptions s JOIN users u ON u.id=s.user_id ORDER BY s.created_at DESC LIMIT 500')->fetchAll(),
            'announcements'=>$pdo->query('SELECT * FROM announcements ORDER BY created_at DESC LIMIT 100')->fetchAll(),
            'banners'=>$pdo->query('SELECT * FROM banners ORDER BY sort_order,id LIMIT 100')->fetchAll(),
            'loader_config'=>$pdo->query('SELECT * FROM loader_config WHERE id=1')->fetch() ?: [],
            'loader_versions'=>$pdo->query('SELECT * FROM loader_versions ORDER BY published_at DESC LIMIT 100')->fetchAll(),
            'downloads'=>$pdo->query('SELECT dl.*,u.email,g.name game_name FROM download_logs dl LEFT JOIN users u ON u.id=dl.user_id LEFT JOIN patch_archives pa ON pa.id=dl.patch_archive_id LEFT JOIN patch_versions pv ON pv.id=pa.patch_version_id LEFT JOIN patches p ON p.id=pv.patch_id LEFT JOIN games g ON g.id=p.game_id ORDER BY dl.created_at DESC LIMIT 200')->fetchAll(),
            'audit'=>$pdo->query('SELECT a.*,u.email FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_user_id ORDER BY a.created_at DESC LIMIT 200')->fetchAll(),
            'templates'=>$pdo->query('SELECT * FROM patch_templates WHERE is_active=1 ORDER BY name')->fetchAll(),
        ];
    }

    public function saveGame(array $in, int $actor): int
    {
        foreach(['name','slug'] as $f) if(trim((string)($in[$f]??''))==='') throw new DomainException("{$f} zorunludur.");
        if(!preg_match('/^[a-z0-9]+(?:-[a-z0-9]+)*$/',$in['slug'])) throw new \DomainException('Slug yalnız küçük harf, sayı ve tire içerebilir.');
        $pdo=Database::connection(); $id=(int)($in['id']??0); $before=null;
        $executable=trim((string)($in['executable']??''))?:null;$process=trim((string)($in['process_name']??''))?:$executable;
        $cover=$in['cover_path']??$in['local_cover_path']??$in['cover_url']??'/assets/placeholders/cover-generic.svg';
        $banner=$in['banner_path']??$in['local_banner_path']??$in['banner_url']??'/assets/placeholders/banner-generic.svg';
        $values=[trim($in['name']),$in['slug'],trim($in['short_description']??''),trim($in['description']??''),($in['cover_url']??'')?:null,($in['banner_url']??'')?:null,($in['local_cover_path']??'')?:null,($in['local_banner_path']??'')?:null,$cover,$banner,($in['steam_app_id']??'')?:null,($in['epic_catalog_id']??'')?:null,$executable,$process,in_array($in['access_type']??'', ['free','premium'],true)?$in['access_type']:'free',max(0,min(100,(int)($in['translation_percent']??0))),$in['minimum_loader_version']??'0.1.0',json_encode($in['supported_stores']??['manual']),!empty($in['is_active'])?1:0];
        $pdo->beginTransaction();
        try {
            if($id){$s=$pdo->prepare('SELECT * FROM games WHERE id=? FOR UPDATE');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new \DomainException('Oyun bulunamadı.');
                $pdo->prepare('UPDATE games SET name=?,slug=?,short_description=?,description=?,cover_url=?,banner_url=?,local_cover_path=?,local_banner_path=?,cover_path=?,banner_path=?,steam_app_id=?,epic_catalog_id=?,executable=?,process_name=?,access_type=?,translation_percent=?,minimum_loader_version=?,supported_stores=?,is_active=? WHERE id=?')->execute([...$values,$id]);
            } else {$pdo->prepare('INSERT INTO games(name,slug,short_description,description,cover_url,banner_url,local_cover_path,local_banner_path,cover_path,banner_path,steam_app_id,epic_catalog_id,executable,process_name,access_type,translation_percent,minimum_loader_version,supported_stores,is_active,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)')->execute([...$values,$actor]);$id=(int)$pdo->lastInsertId();}
            if(isset($in['required_files'])){$pdo->prepare("DELETE FROM game_detection_rules WHERE game_id=? AND rule_type IN ('required_file','optional_file')")->execute([$id]); foreach(['required_file'=>'required_files','optional_file'=>'optional_files'] as $type=>$field) foreach(array_filter(array_map('trim',(array)($in[$field]??[]))) as $i=>$path){if(!PathGuard::isSafeRelative($path))throw new \DomainException('Güvenli olmayan tespit yolu: '.$path);$pdo->prepare('INSERT INTO game_detection_rules(game_id,provider,rule_type,rule_value,sort_order,is_required) VALUES(?,?,?,?,?,?)')->execute([$id,'manual',$type,$path,($i+1)*10,$type==='required_file']);}}
            if(array_key_exists('category_ids',$in)){$pdo->prepare('DELETE FROM game_categories WHERE game_id=?')->execute([$id]);$categoryIds=array_values(array_unique(array_filter(array_map('intval',(array)$in['category_ids']))));if($categoryIds){$check=$pdo->prepare('SELECT id FROM categories WHERE id=?');$insert=$pdo->prepare('INSERT INTO game_categories(game_id,category_id) VALUES(?,?)');foreach($categoryIds as $categoryId){$check->execute([$categoryId]);if(!$check->fetchColumn())throw new DomainException('Kategori bulunamadı.');$insert->execute([$id,$categoryId]);}}}
            $pdo->commit(); (new AuditService())->write($actor,$before?'game.updated':'game.created','game',$id,$before,$in); return $id;
        } catch(\Throwable $e){$pdo->rollBack();throw $e;}
    }

    public function duplicateGame(int $id,int $actor): int
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM games WHERE id=?');$s->execute([$id]);$g=$s->fetch();if(!$g)throw new \DomainException('Oyun bulunamadı.');
        $base=$g['slug'].'-kopya';$slug=$base;$n=2;while($this->slugExists($slug))$slug=$base.'-'.$n++;
        $g['name'].=' - Kopya';$g['slug']=$slug;$g['is_active']=0;$g['id']=null;
        $new=$this->saveGame($g,$actor);
        $pdo->prepare('INSERT INTO game_detection_rules(game_id,provider,rule_type,rule_value,expected_hash,sort_order,is_required) SELECT ?,provider,rule_type,rule_value,expected_hash,sort_order,is_required FROM game_detection_rules WHERE game_id=?')->execute([$new,$id]);
        return $new;
    }

    public function createPatchVersion(array $in,array $file,int $actor): int
    {
        foreach(['game_id','version'] as $f)if(empty($in[$f]))throw new \DomainException("{$f} zorunludur.");
        if(!preg_match('/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/',$in['version']))throw new \DomainException('Patch version SemVer olmalıdır.');
        $sourceType=($in['source_type']??'server')==='external'?'external':'server';
        $archive=null;
        if($sourceType==='external'){
            $externalUrl=trim((string)($in['external_url']??''));
            $sha256=strtolower(trim((string)($in['sha256']??'')));
            $sizeBytes=(int)($in['size_bytes']??0);
            $originalName=trim((string)($in['original_name']??''))?:'external-patch.zip';
            $this->assertSafeExternalUrl($externalUrl);
            if(!preg_match('/^[a-f0-9]{64}$/',$sha256))throw new DomainException('Harici patch için geçerli SHA-256 zorunludur.');
            if($sizeBytes<1)throw new DomainException('Harici patch için dosya boyutu (byte) zorunludur.');
            if(!preg_match('/\.zip$/i',$originalName))$originalName.='.zip';
            $archive=['storage_name'=>'external-'.bin2hex(random_bytes(16)).'.zip','original_name'=>basename(str_replace('\\','/',$originalName)),'mime_type'=>'application/zip','sha256'=>$sha256,'size_bytes'=>$sizeBytes,'file_tree'=>[],'external_url'=>$externalUrl];
        }else{
            $archive=(new PatchStorage())->storeUpload($file);
            $archive['external_url']=null;
        }
        $pdo=Database::connection();$pdo->beginTransaction();
        try{$s=$pdo->prepare('SELECT id FROM patches WHERE game_id=? LIMIT 1');$s->execute([(int)$in['game_id']]);$patchId=(int)($s->fetchColumn()?:0);if(!$patchId){$pdo->prepare('INSERT INTO patches(game_id,name) VALUES(?,?)')->execute([(int)$in['game_id'],'Türkçe Yama']);$patchId=(int)$pdo->lastInsertId();}
            $pdo->prepare('INSERT INTO patch_versions(patch_id,version,game_version,changelog,minimum_loader_version,status,channel,mandatory_update,access_type,schema_version,created_by) VALUES(?,?,?,?,?,?,?,?,?,1,?)')->execute([$patchId,$in['version'],$in['game_version']??null,$in['changelog']??null,$in['minimum_loader_version']??'0.1.0','DRAFT',in_array($in['channel']??'', ['stable','beta','internal'],true)?$in['channel']:'internal',!empty($in['mandatory_update'])?1:0,in_array($in['access_type']??'', ['free','premium'],true)?$in['access_type']:'free',$actor]);$versionId=(int)$pdo->lastInsertId();
            $pdo->prepare('INSERT INTO patch_archives(patch_version_id,source_type,external_url,storage_name,original_name,mime_type,sha256,size_bytes,file_tree) VALUES(?,?,?,?,?,?,?,?,?)')->execute([$versionId,$sourceType,$archive['external_url'],$archive['storage_name'],$archive['original_name'],$archive['mime_type'],$archive['sha256'],$archive['size_bytes'],json_encode($archive['file_tree'],JSON_UNESCAPED_SLASHES)]);$pdo->commit();(new AuditService())->write($actor,'patch.created','patch_version',$versionId,null,$in);return $versionId;
        }catch(\Throwable $e){$pdo->rollBack();if($sourceType==='server'&&!empty($archive['storage_name']))@unlink((new PatchStorage())->path($archive['storage_name']));throw $e;}
    }

    private function assertSafeExternalUrl(string $url): void
    {
        if($url===''||!filter_var($url,FILTER_VALIDATE_URL))throw new DomainException("Geçerli harici indirme URL'si zorunludur.");
        $parts=parse_url($url);if(strtolower((string)($parts['scheme']??''))!=='https')throw new DomainException('Harici patch URL yalnız HTTPS olabilir.');
        $host=strtolower((string)($parts['host']??''));if($host===''||$host==='localhost'||str_ends_with($host,'.localhost'))throw new DomainException('Yerel ağ adresleri harici patch kaynağı olamaz.');
        $ips=filter_var($host,FILTER_VALIDATE_IP)?[$host]:(gethostbynamel($host)?:[]);
        foreach($ips as $ip)if(!filter_var($ip,FILTER_VALIDATE_IP,FILTER_FLAG_NO_PRIV_RANGE|FILTER_FLAG_NO_RES_RANGE))throw new DomainException('Private/rezerve IP adreslerine harici patch bağlantısı verilemez.');
    }

    public function saveActions(int $versionId,array $actions,int $actor): void
    {
        $manifestActions=[];foreach($actions as $i=>$a){$type=$a['type']??'';$id=$a['id']??$this->uuid();$manifestActions[]=['id'=>$id,'type'=>$type,'source'=>$a['source']??null,'destination'=>$a['destination']??'','backup'=>(bool)($a['backup']??true)];}
        $errors=(new ManifestValidator())->validate(['schema_version'=>1,'game'=>['id'=>1,'slug'=>'validation'],'detection'=>['executable'=>'Validation.exe','required_files'=>[]],'patch'=>['version'=>'0.0.0'],'archive'=>['sha256'=>str_repeat('0',64),'size'=>1],'install_actions'=>$manifestActions,'integrity'=>[],'backup'=>[]]);
        if($errors)throw new \DomainException(implode(' ',$errors));
        $pdo=Database::connection();$pdo->beginTransaction();try{$pdo->prepare('DELETE FROM patch_install_actions WHERE patch_version_id=?')->execute([$versionId]);$stmt=$pdo->prepare('INSERT INTO patch_install_actions(patch_version_id,action_uuid,action_type,source_path,destination_path,backup_enabled,expected_sha256,sort_order,options_json) VALUES(?,?,?,?,?,?,?,?,?)');foreach($manifestActions as $i=>$a)$stmt->execute([$versionId,$a['id'],$a['type'],$a['source']?:null,$a['destination'],$a['backup']?1:0,$actions[$i]['expected_sha256']??null,($i+1)*10,json_encode($actions[$i]['options']??new \stdClass())]);$pdo->commit();(new AuditService())->write($actor,'actions.updated','patch_version',$versionId,null,$manifestActions);}catch(\Throwable$e){$pdo->rollBack();throw$e;}
    }

    public function publish(int $versionId,int $actor): void
    {
        $manifest=(new ManifestService())->build($versionId);$errors=(new ManifestValidator())->validate($manifest);if($errors)throw new \DomainException('Yayın kontrolü başarısız: '.implode(' ',$errors));
        $pdo=Database::connection();$pdo->beginTransaction();try{$s=$pdo->prepare('SELECT patch_id,channel FROM patch_versions WHERE id=? FOR UPDATE');$s->execute([$versionId]);$v=$s->fetch();if(!$v)throw new \DomainException('Sürüm bulunamadı.');$pdo->prepare("UPDATE patch_versions SET status=IF(id=?,'PUBLISHED',IF(status='PUBLISHED','ARCHIVED',status)),manifest_snapshot=IF(id=?,?,manifest_snapshot),published_at=IF(id=?,NOW(),published_at) WHERE patch_id=? AND channel=?")->execute([$versionId,$versionId,json_encode($manifest,JSON_UNESCAPED_UNICODE|JSON_UNESCAPED_SLASHES),$versionId,$v['patch_id'],$v['channel']]);$pdo->prepare('INSERT INTO patch_release_channels(patch_id,channel,active_patch_version_id,updated_by) VALUES(?,?,?,?) ON DUPLICATE KEY UPDATE active_patch_version_id=VALUES(active_patch_version_id),updated_by=VALUES(updated_by)')->execute([$v['patch_id'],$v['channel'],$versionId,$actor]);$pdo->commit();(new AuditService())->write($actor,'patch.published','patch_version',$versionId,null,['channel'=>$v['channel']]);}catch(\Throwable$e){$pdo->rollBack();throw$e;}
    }

    public function rollbackPatch(int $versionId,int $actor): void
    {
        $s=Database::connection()->prepare('SELECT status FROM patch_versions WHERE id=?');$s->execute([$versionId]);$status=$s->fetchColumn();
        if($status===false)throw new DomainException('Patch sürümü bulunamadı.');
        if(!in_array($status,['ARCHIVED','PUBLISHED'],true))throw new DomainException('Yalnız daha önce yayınlanmış bir sürüme rollback yapılabilir.');
        $this->publish($versionId,$actor);(new AuditService())->write($actor,'patch.rollback','patch_version',$versionId,['status'=>$status],['status'=>'PUBLISHED']);
    }

    public function saveLoaderConfig(array $in,int $actor): void
    {
        $color=$in['accent_color']??'#B7F34A';if(!preg_match('/^#[0-9A-Fa-f]{6}$/',$color))throw new \DomainException('Accent color geçersiz.');
        foreach(['logo_url','banner_url','discord_url','youtube_url','instagram_url','x_url','support_url'] as $k)if(!empty($in[$k])&&!filter_var($in[$k],FILTER_VALIDATE_URL)&&!str_starts_with($in[$k],'/'))throw new \DomainException("{$k} geçerli URL olmalıdır.");
        Database::connection()->prepare('INSERT INTO loader_config(id,app_name,logo_url,banner_url,accent_color,library_title,discord_url,youtube_url,instagram_url,x_url,support_url) VALUES(1,?,?,?,?,?,?,?,?,?,?) ON DUPLICATE KEY UPDATE app_name=VALUES(app_name),logo_url=VALUES(logo_url),banner_url=VALUES(banner_url),accent_color=VALUES(accent_color),library_title=VALUES(library_title),discord_url=VALUES(discord_url),youtube_url=VALUES(youtube_url),instagram_url=VALUES(instagram_url),x_url=VALUES(x_url),support_url=VALUES(support_url)')->execute([$in['app_name']??'Türkçe Yama Loader',$in['logo_url']??null,$in['banner_url']??null,$color,$in['library_title']??'Kütüphane',$in['discord_url']??null,$in['youtube_url']??null,$in['instagram_url']??null,$in['x_url']??null,$in['support_url']??null]);(new AuditService())->write($actor,'loader_config.updated','loader_config',1,null,$in);
    }

    public function saveBrandingMedia(array $input,array $files,int $actor): array
    {
        $slot=in_array($input['slot']??'', ['login','library'],true)?$input['slot']:throw new DomainException('Arka plan alanı geçersiz.');
        $type=in_array($input['background_type']??'', ['default','image','video'],true)?$input['background_type']:throw new DomainException('Arka plan türü geçersiz.');
        $overlay=max(0,min(100,(int)($input['overlay']??($slot==='login'?60:55))));$prefix=$slot.'_background_';$storage=new BrandingMediaStorage();$pdo=Database::connection();
        $old=$pdo->query('SELECT * FROM loader_config WHERE id=1')->fetch();if(!$old)throw new DomainException('Loader config bulunamadı.');
        $image=$old[$prefix.'image']??null;$video=$old[$prefix.'video']??null;$fallback=$old[$prefix.'fallback']??null;$created=[];
        try{
            if($type==='default'){$image=null;$video=null;$fallback=null;}
            elseif($type==='image'){
                if(($files['media']['error']??UPLOAD_ERR_NO_FILE)===UPLOAD_ERR_OK){$upload=$storage->storeUpload($files['media'],'image');$created[]=$upload['url'];$image=$upload['url'];}
                if(!$image)throw new DomainException('Resim türü için JPG, PNG veya WebP yükleyin.');
                $video=null;$fallback=null;
            }else{
                if(($files['media']['error']??UPLOAD_ERR_NO_FILE)===UPLOAD_ERR_OK){$upload=$storage->storeUpload($files['media'],'video');$created[]=$upload['url'];$video=$upload['url'];}
                if(!$video)throw new DomainException('Video türü için MP4 veya WebM yükleyin.');
                if(($files['fallback']['error']??UPLOAD_ERR_NO_FILE)===UPLOAD_ERR_OK){$upload=$storage->storeUpload($files['fallback'],'image');$created[]=$upload['url'];$fallback=$upload['url'];}
                elseif(!$fallback&&($old[$prefix.'type']??'default')==='image'&&$image){$fallback=$image;}
                $image=null;
            }
            $legacy=$slot==='login'?($type==='image'?$image:($type==='video'?$fallback:null)):($old['login_background_url']??null);
            $pdo->beginTransaction();$pdo->prepare("UPDATE loader_config SET {$prefix}type=?,{$prefix}image=?,{$prefix}video=?,{$prefix}fallback=?,{$prefix}overlay=?,login_background_url=? WHERE id=1")->execute([$type,$image,$video,$fallback,$overlay,$legacy]);$pdo->commit();
        }catch(Throwable $error){if($pdo->inTransaction())$pdo->rollBack();foreach($created as $url)$storage->deleteManagedUrl($url);throw $error;}
        $after=['type'=>$type,'image'=>$image,'video'=>$video,'fallback'=>$fallback,'overlay'=>$overlay];(new AuditService())->write($actor,'branding.'.$slot.'.updated','loader_config',1,['type'=>$old[$prefix.'type']??'default','image'=>$old[$prefix.'image']??null,'video'=>$old[$prefix.'video']??null,'fallback'=>$old[$prefix.'fallback']??null,'overlay'=>$old[$prefix.'overlay']??null],$after);
        foreach(array_unique(array_filter([$old[$prefix.'image']??null,$old[$prefix.'video']??null,$old[$prefix.'fallback']??null])) as $url)$this->deleteBrandingIfUnused((string)$url,$storage);
        return $after;
    }

    public function resetBrandingMedia(string $slot,int $actor): array
    {
        return $this->saveBrandingMedia(['slot'=>$slot,'background_type'=>'default','overlay'=>$slot==='login'?60:55],[],$actor);
    }

    public function setGameStatus(int $id,bool $active,int $actor): void
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT id,name,is_active FROM games WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Oyun bulunamadı.');
        $pdo->prepare('UPDATE games SET is_active=? WHERE id=?')->execute([$active?1:0,$id]);
        (new AuditService())->write($actor,$active?'game.activated':'game.deactivated','game',$id,$before,['is_active'=>$active]);
    }

    public function deleteGame(int $id,int $actor): void
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM games WHERE id=?');$s->execute([$id]);$game=$s->fetch();if(!$game)throw new DomainException('Oyun bulunamadı.');
        $s=$pdo->prepare('SELECT pa.storage_name FROM patch_archives pa JOIN patch_versions pv ON pv.id=pa.patch_version_id JOIN patches p ON p.id=pv.patch_id WHERE p.game_id=?');$s->execute([$id]);$archives=$s->fetchAll(PDO::FETCH_COLUMN);
        $pdo->beginTransaction();try{$pdo->prepare('DELETE prc FROM patch_release_channels prc JOIN patches p ON p.id=prc.patch_id WHERE p.game_id=?')->execute([$id]);$pdo->prepare('DELETE FROM games WHERE id=?')->execute([$id]);(new AuditService())->write($actor,'game.deleted','game',$id,$game,null);$pdo->commit();}catch(Throwable $error){$pdo->rollBack();throw $error;}
        $images=new ImageStorage();foreach(['local_cover_path','local_banner_path','local_icon_path'] as $column)$images->deleteManaged($game[$column]??null);
        $storage=new PatchStorage();foreach($archives as $name){$path=$storage->path((string)$name);if(is_file($path))unlink($path);}
    }

    public function saveGameImage(int $gameId,string $kind,array $file,int $actor): string
    {
        if(!in_array($kind,['cover','banner','icon'],true))throw new DomainException('Görsel türü geçersiz.');
        $pdo=Database::connection();$s=$pdo->prepare('SELECT id,local_cover_path,local_banner_path,local_icon_path FROM games WHERE id=?');$s->execute([$gameId]);$game=$s->fetch();if(!$game)throw new DomainException('Oyun bulunamadı.');
        $columns=['cover'=>['local_cover_path','cover_path'],'banner'=>['local_banner_path','banner_path'],'icon'=>['local_icon_path','icon_path']];[$localColumn,$displayColumn]=$columns[$kind];$storage=new ImageStorage();$path=$storage->store($file,$kind);$previous=$game[$localColumn]??null;
        try{$pdo->prepare("UPDATE games SET {$localColumn}=?,{$displayColumn}=? WHERE id=?")->execute([$path,$path,$gameId]);}catch(Throwable $e){$storage->deleteManaged($path);throw $e;}
        $storage->deleteManaged($previous);(new AuditService())->write($actor,'game.image.updated','game',$gameId,[$localColumn=>$previous],[$localColumn=>$path]);return $path;
    }

    public function deleteGameImage(int $gameId,string $kind,int $actor): void
    {
        if(!in_array($kind,['cover','banner','icon'],true))throw new DomainException('Görsel türü geçersiz.');
        $pdo=Database::connection();$columns=['cover'=>['local_cover_path','cover_path','/assets/placeholders/cover-generic.svg'],'banner'=>['local_banner_path','banner_path','/assets/placeholders/banner-generic.svg'],'icon'=>['local_icon_path','icon_path','/assets/placeholders/icon-generic.svg']];[$localColumn,$displayColumn,$fallback]=$columns[$kind];
        $s=$pdo->prepare("SELECT {$localColumn} FROM games WHERE id=?");$s->execute([$gameId]);$previous=$s->fetchColumn();if($previous===false)throw new DomainException('Oyun bulunamadı.');
        $pdo->prepare("UPDATE games SET {$localColumn}=NULL,{$displayColumn}=? WHERE id=?")->execute([$fallback,$gameId]);(new ImageStorage())->deleteManaged($previous?:null);
        (new AuditService())->write($actor,'game.image.deleted','game',$gameId,[$localColumn=>$previous],[$localColumn=>null]);
    }

    public function builderData(int $versionId): array
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT pv.id,pv.status,pv.version,pa.file_tree,pa.sha256,pa.size_bytes,pa.original_name,pa.source_type,pa.external_url FROM patch_versions pv LEFT JOIN patch_archives pa ON pa.patch_version_id=pv.id WHERE pv.id=?');$s->execute([$versionId]);$version=$s->fetch();if(!$version)throw new DomainException('Patch sürümü bulunamadı.');
        $s=$pdo->prepare('SELECT action_uuid id,action_type type,source_path source,destination_path destination,backup_enabled backup,expected_sha256,options_json options FROM patch_install_actions WHERE patch_version_id=? ORDER BY sort_order,id');$s->execute([$versionId]);$actions=$s->fetchAll();
        foreach($actions as &$action){$action['backup']=(bool)$action['backup'];$action['options']=json_decode($action['options']??'{}',true)?:[];}unset($action);
        $version['file_tree']=json_decode($version['file_tree']??'[]',true)?:[];$version['actions']=$actions;return $version;
    }

    public function setPatchStatus(int $versionId,string $status,int $actor): void
    {
        if(!in_array($status,['DRAFT','TESTING','DISABLED','ARCHIVED'],true))throw new DomainException('Patch durumu geçersiz.');
        $pdo=Database::connection();$s=$pdo->prepare('SELECT id,status FROM patch_versions WHERE id=?');$s->execute([$versionId]);$before=$s->fetch();if(!$before)throw new DomainException('Patch sürümü bulunamadı.');
        $pdo->prepare('UPDATE patch_versions SET status=? WHERE id=?')->execute([$status,$versionId]);(new AuditService())->write($actor,'patch.status.updated','patch_version',$versionId,$before,['status'=>$status]);
    }

    public function saveCategory(array $input,int $actor): int
    {
        $name=trim((string)($input['name']??''));$slug=trim((string)($input['slug']??''));
        if($name===''||!preg_match('/^[a-z0-9]+(?:-[a-z0-9]+)*$/',$slug))throw new DomainException('Kategori adı ve güvenli slug zorunludur.');
        $id=(int)($input['id']??0);$pdo=Database::connection();$before=null;
        if($id){$s=$pdo->prepare('SELECT * FROM categories WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Kategori bulunamadı.');$pdo->prepare('UPDATE categories SET name=?,slug=?,sort_order=?,is_active=? WHERE id=?')->execute([$name,$slug,(int)($input['sort_order']??0),!empty($input['is_active'])?1:0,$id]);}
        else{$pdo->prepare('INSERT INTO categories(name,slug,sort_order,is_active) VALUES(?,?,?,?)')->execute([$name,$slug,(int)($input['sort_order']??0),!empty($input['is_active'])?1:0]);$id=(int)$pdo->lastInsertId();}
        (new AuditService())->write($actor,$before?'category.updated':'category.created','category',$id,$before,$input);return $id;
    }

    public function deleteCategory(int $id,int $actor): void
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM categories WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Kategori bulunamadı.');
        $pdo->prepare('DELETE FROM categories WHERE id=?')->execute([$id]);(new AuditService())->write($actor,'category.deleted','category',$id,$before,null);
    }

    public function saveAnnouncement(array $input,int $actor): int
    {
        $title=trim((string)($input['title']??''));$body=trim((string)($input['body']??''));$audience=in_array($input['audience']??'all',['all','free','premium','tester','admin'],true)?$input['audience']:'all';
        if($title===''||$body==='')throw new DomainException('Duyuru başlığı ve metni zorunludur.');
        $id=(int)($input['id']??0);$pdo=Database::connection();$before=null;$values=[$title,$body,$audience,!empty($input['is_active'])?1:0,$this->dateValue($input['starts_at']??null),$this->dateValue($input['ends_at']??null)];
        if($id){$s=$pdo->prepare('SELECT * FROM announcements WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Duyuru bulunamadı.');$pdo->prepare('UPDATE announcements SET title=?,body=?,audience=?,is_active=?,starts_at=?,ends_at=? WHERE id=?')->execute([...$values,$id]);}
        else{$pdo->prepare('INSERT INTO announcements(title,body,audience,is_active,starts_at,ends_at) VALUES(?,?,?,?,?,?)')->execute($values);$id=(int)$pdo->lastInsertId();}
        (new AuditService())->write($actor,$before?'announcement.updated':'announcement.created','announcement',$id,$before,$input);return $id;
    }

    public function deleteAnnouncement(int $id,int $actor): void
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM announcements WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Duyuru bulunamadı.');$pdo->prepare('DELETE FROM announcements WHERE id=?')->execute([$id]);(new AuditService())->write($actor,'announcement.deleted','announcement',$id,$before,null);
    }

    public function saveBanner(array $input,array $file,int $actor): int
    {
        $title=trim((string)($input['title']??''));if($title==='')throw new DomainException('Banner başlığı zorunludur.');
        $target=trim((string)($input['target_url']??''))?:null;if($target&&!filter_var($target,FILTER_VALIDATE_URL)&&!str_starts_with($target,'/'))throw new DomainException('Banner hedef URL geçersiz.');
        $path=(new ImageStorage())->store($file,'banner');$pdo=Database::connection();
        try{$pdo->prepare('INSERT INTO banners(title,image_path,target_url,sort_order,is_active) VALUES(?,?,?,?,?)')->execute([$title,$path,$target,(int)($input['sort_order']??0),!empty($input['is_active'])?1:0]);$id=(int)$pdo->lastInsertId();}catch(Throwable $error){(new ImageStorage())->deleteManaged($path);throw $error;}
        (new AuditService())->write($actor,'banner.created','banner',$id,null,$input);return $id;
    }

    public function deleteBanner(int $id,int $actor): void
    {
        $pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM banners WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Banner bulunamadı.');$pdo->prepare('DELETE FROM banners WHERE id=?')->execute([$id]);(new ImageStorage())->deleteManaged($before['image_path']??null);(new AuditService())->write($actor,'banner.deleted','banner',$id,$before,null);
    }

    public function saveSubscription(array $input,int $actor): int
    {
        $userId=(int)($input['user_id']??0);$plan=trim((string)($input['plan_name']??''));$status=in_array($input['status']??'', ['active','expired','cancelled','trial'],true)?$input['status']:'active';
        if(!$userId||$plan==='')throw new DomainException('Kullanıcı ve plan adı zorunludur.');
        $starts=$this->dateValue($input['starts_at']??null)??date('Y-m-d H:i:s');$ends=$this->dateValue($input['ends_at']??null);
        $pdo=Database::connection();$pdo->prepare('INSERT INTO subscriptions(user_id,plan_name,status,starts_at,ends_at) VALUES(?,?,?,?,?)')->execute([$userId,$plan,$status,$starts,$ends]);$id=(int)$pdo->lastInsertId();(new AuditService())->write($actor,'subscription.created','subscription',$id,null,$input);return $id;
    }

    public function setSubscriptionStatus(int $id,string $status,int $actor): void
    {
        if(!in_array($status,['active','expired','cancelled','trial'],true))throw new DomainException('Abonelik durumu geçersiz.');$pdo=Database::connection();$s=$pdo->prepare('SELECT * FROM subscriptions WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Abonelik bulunamadı.');$pdo->prepare('UPDATE subscriptions SET status=? WHERE id=?')->execute([$status,$id]);(new AuditService())->write($actor,'subscription.status.updated','subscription',$id,$before,['status'=>$status]);
    }

    public function updateUser(array $input,int $actor): void
    {
        $id=(int)($input['id']??0);$role=in_array($input['role']??'', ['user','tester','admin','super_admin'],true)?$input['role']:'user';$channel=in_array($input['release_channel']??'', ['stable','beta','internal'],true)?$input['release_channel']:'stable';$status=in_array($input['status']??'', ['active','suspended','pending'],true)?$input['status']:'active';
        if($id===$actor&&($status!=='active'||!in_array($role,['admin','super_admin'],true)))throw new DomainException('Kendi aktif admin yetkinizi kaldıramazsınız.');
        $pdo=Database::connection();$s=$pdo->prepare('SELECT id,email,role,release_channel,status FROM users WHERE id=?');$s->execute([$id]);$before=$s->fetch();if(!$before)throw new DomainException('Kullanıcı bulunamadı.');$pdo->prepare('UPDATE users SET role=?,release_channel=?,status=? WHERE id=?')->execute([$role,$channel,$status,$id]);(new AuditService())->write($actor,'user.updated','user',$id,$before,$input);
    }

    public function createLoaderVersion(array $input,array $file,int $actor): int
    {
        $version=trim((string)($input['version']??''));if(!preg_match('/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/',$version))throw new DomainException('Loader version SemVer olmalıdır.');
        $channel=in_array($input['channel']??'', ['stable','beta','internal'],true)?$input['channel']:'internal';$archive=(new LoaderStorage())->storeUpload($file);$pdo=Database::connection();
        try{$pdo->prepare('INSERT INTO loader_versions(version,channel,storage_name,sha256,size_bytes,mandatory,release_notes) VALUES(?,?,?,?,?,?,?)')->execute([$version,$channel,$archive['storage_name'],$archive['sha256'],$archive['size_bytes'],!empty($input['mandatory'])?1:0,$input['release_notes']??null]);$id=(int)$pdo->lastInsertId();}catch(Throwable $error){@unlink((new LoaderStorage())->path($archive['storage_name']));throw $error;}
        (new AuditService())->write($actor,'loader_version.created','loader_version',$id,null,['version'=>$version,'channel'=>$channel,'sha256'=>$archive['sha256']]);return $id;
    }

    private function dateValue(mixed $value): ?string
    {
        $value=trim((string)$value);if($value==='')return null;$date=\DateTimeImmutable::createFromFormat('Y-m-d\TH:i',$value)?:\DateTimeImmutable::createFromFormat('Y-m-d H:i:s',$value);if(!$date)throw new DomainException('Tarih biçimi geçersiz.');return $date->format('Y-m-d H:i:s');
    }

    private function deleteBrandingIfUnused(string $url,BrandingMediaStorage $storage): void
    {
        $s=Database::connection()->prepare('SELECT 1 FROM loader_config WHERE login_background_image=? OR login_background_video=? OR login_background_fallback=? OR library_background_image=? OR library_background_video=? OR library_background_fallback=? LIMIT 1');$s->execute(array_fill(0,6,$url));if(!$s->fetchColumn())$storage->deleteManagedUrl($url);
    }

    private function slugExists(string $slug):bool{$s=Database::connection()->prepare('SELECT 1 FROM games WHERE slug=?');$s->execute([$slug]);return(bool)$s->fetchColumn();}
    private function uuid():string{$d=random_bytes(16);$d[6]=chr((ord($d[6])&0x0f)|0x40);$d[8]=chr((ord($d[8])&0x3f)|0x80);return vsprintf('%s%s-%s-%s-%s-%s%s%s',str_split(bin2hex($d),4));}
}
