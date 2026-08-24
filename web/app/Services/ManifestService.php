<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;
use App\Core\Env;

final class ManifestService
{
    public function build(int $versionId): array
    {
        $pdo=Database::connection();
        $stmt=$pdo->prepare('SELECT pv.*,p.game_id,g.slug,g.name game_name,g.steam_app_id,g.epic_catalog_id,g.executable,g.process_name,pa.sha256,pa.size_bytes FROM patch_versions pv JOIN patches p ON p.id=pv.patch_id JOIN games g ON g.id=p.game_id LEFT JOIN patch_archives pa ON pa.patch_version_id=pv.id WHERE pv.id=?');
        $stmt->execute([$versionId]); $row=$stmt->fetch();
        if (!$row) throw new \DomainException('Patch sürümü bulunamadı.');
        $stmt=$pdo->prepare('SELECT provider,rule_type,rule_value,is_required,expected_hash FROM game_detection_rules WHERE game_id=? ORDER BY sort_order,id');
        $stmt->execute([$row['game_id']]); $rules=$stmt->fetchAll();
        $required=[]; $optional=[];
        foreach($rules as $rule) {
            if($rule['rule_type']==='required_file') $required[]=$rule['rule_value'];
            if($rule['rule_type']==='optional_file') $optional[]=$rule['rule_value'];
        }
        $stmt=$pdo->prepare('SELECT action_uuid,action_type,source_path,destination_path,backup_enabled,expected_sha256,options_json FROM patch_install_actions WHERE patch_version_id=? ORDER BY sort_order,id');
        $stmt->execute([$versionId]);
        $actions=array_map(static fn(array $a)=>array_filter([
            'id'=>$a['action_uuid'],'type'=>$a['action_type'],'source'=>$a['source_path'],'destination'=>$a['destination_path'],
            'backup'=>(bool)$a['backup_enabled'],'expected_sha256'=>$a['expected_sha256'],'options'=>json_decode($a['options_json'] ?? '{}',true) ?: (object)[],
        ],static fn($v)=>$v!==null),$stmt->fetchAll());
        return [
            'schema_version'=>(int)$row['schema_version'],
            'game'=>['id'=>(int)$row['game_id'],'slug'=>$row['slug'],'name'=>$row['game_name']],
            'detection'=>['steam_app_id'=>$row['steam_app_id'],'epic_catalog_id'=>$row['epic_catalog_id'],'executable'=>(string)($row['executable']??''),'process_name'=>$row['process_name'],'required_files'=>$required,'optional_files'=>$optional],
            'patch'=>['id'=>(int)$row['id'],'version'=>$row['version'],'game_version'=>$row['game_version'],'minimum_loader_version'=>$row['minimum_loader_version'],'mandatory'=>(bool)$row['mandatory_update'],'channel'=>$row['channel']],
            'archive'=>['download_token_url'=>rtrim(Env::get('APP_URL',''),'/').'/api/patches/'.$row['id'].'/download-token','sha256'=>$row['sha256'],'size'=>(int)$row['size_bytes']],
            'install_actions'=>$actions,
            'integrity'=>['verify_after_install'=>true,'conflict_policy'=>'abort'],
            'backup'=>['automatic'=>true,'retain_until_uninstall'=>true],
        ];
    }

    public function validateVersion(int $versionId): array
    {
        return (new ManifestValidator())->validate($this->build($versionId));
    }
}
