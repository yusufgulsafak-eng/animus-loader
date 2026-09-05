<?php
declare(strict_types=1);

namespace App\Services;

use App\Core\Database;

final class CatalogService
{
    public function games(array $user, array $filters = []): array
    {
        $channel = $user['release_channel'] ?? 'stable';
        $sql = "SELECT g.*, GROUP_CONCAT(DISTINCT c.name ORDER BY c.name SEPARATOR ', ') categories,
                pv.id patch_version_id,pv.version patch_version,pv.game_version,pv.changelog,pv.channel,pv.published_at,pv.updated_at patch_updated_at,pa.size_bytes
                FROM games g
                LEFT JOIN game_categories gc ON gc.game_id=g.id LEFT JOIN categories c ON c.id=gc.category_id
                LEFT JOIN patches p ON p.game_id=g.id
                LEFT JOIN patch_release_channels prc ON prc.patch_id=p.id AND prc.channel=?
                LEFT JOIN patch_versions pv ON pv.id=prc.active_patch_version_id AND pv.status='PUBLISHED'
                LEFT JOIN patch_archives pa ON pa.patch_version_id=pv.id
                WHERE g.is_active=1";
        $params = [$channel];
        if (!empty($filters['q'])) { $sql .= ' AND (g.name LIKE ? OR g.slug LIKE ? OR g.steam_app_id LIKE ?)'; $q='%'.$filters['q'].'%'; array_push($params,$q,$q,$q); }
        if (!empty($filters['access']) && in_array($filters['access'], ['free','premium'], true)) { $sql .= ' AND g.access_type=?'; $params[]=$filters['access']; }
        $sql .= ' GROUP BY g.id,pv.id,pa.id ORDER BY g.name LIMIT 500';
        $stmt=Database::connection()->prepare($sql); $stmt->execute($params);
        return array_map([$this,'shapeGame'], $stmt->fetchAll());
    }

    public function publicGames(array $filters = []): array
    {
        $sql = "SELECT
                    g.id,g.name,g.slug,g.short_description,g.cover_url,g.banner_url,
                    g.cover_path,g.banner_path,g.local_cover_path,g.local_banner_path,
                    g.access_type,g.translation_percent,g.supported_stores
                FROM games g
                WHERE g.is_active=1";
        $params = [];
        if (!empty($filters['q'])) {
            $sql .= ' AND (g.name LIKE ? OR g.slug LIKE ?)';
            $q = '%'.$filters['q'].'%';
            $params[] = $q;
            $params[] = $q;
        }
        if (!empty($filters['access']) && in_array($filters['access'], ['free','premium'], true)) {
            $sql .= ' AND g.access_type=?';
            $params[] = $filters['access'];
        }
        $sql .= ' ORDER BY g.name LIMIT 500';
        $stmt = Database::connection()->prepare($sql);
        $stmt->execute($params);

        return array_map(static function(array $row): array {
            $cover = $row['local_cover_path'] ?: ($row['cover_url'] ?: ($row['cover_path'] ?: '/assets/placeholders/cover-generic.svg'));
            $banner = $row['local_banner_path'] ?: ($row['banner_url'] ?: ($row['banner_path'] ?: '/assets/placeholders/banner-generic.svg'));
            return [
                'id' => (int)$row['id'],
                'name' => (string)$row['name'],
                'slug' => (string)$row['slug'],
                'short_description' => (string)($row['short_description'] ?? ''),
                'cover_path' => $cover,
                'banner_path' => $banner,
                'access_type' => in_array($row['access_type'] ?? 'free', ['free','premium'], true) ? $row['access_type'] : 'free',
                'translation_percent' => max(0, min(100, (int)($row['translation_percent'] ?? 0))),
                'supported_stores' => json_decode($row['supported_stores'] ?? '[]', true) ?: [],
            ];
        }, $stmt->fetchAll());
    }

    public function game(int $id, array $user): ?array
    {
        foreach ($this->games($user) as $game) if ((int)$game['id']===$id) {
            $stmt=Database::connection()->prepare('SELECT provider,rule_type,rule_value,expected_hash,is_required,sort_order FROM game_detection_rules WHERE game_id=? ORDER BY sort_order,id');
            $stmt->execute([$id]); $game['detection_rules']=$stmt->fetchAll();
            return $game;
        }
        return null;
    }

    public function activePatch(int $gameId, array $user): ?array
    {
        $stmt=Database::connection()->prepare("SELECT pv.*,p.game_id,pa.sha256,pa.size_bytes,pa.original_name FROM patches p JOIN patch_release_channels rc ON rc.patch_id=p.id AND rc.channel=? JOIN patch_versions pv ON pv.id=rc.active_patch_version_id AND pv.status='PUBLISHED' JOIN patch_archives pa ON pa.patch_version_id=pv.id WHERE p.game_id=? LIMIT 1");
        $stmt->execute([$user['release_channel'] ?? 'stable',$gameId]);
        return $stmt->fetch() ?: null;
    }

    private function shapeGame(array $row): array
    {
        $row['id']=(int)$row['id']; $row['translation_percent']=(int)$row['translation_percent']; $row['is_active']=(bool)$row['is_active'];
        $row['supported_stores']=json_decode($row['supported_stores'] ?? '[]',true) ?: [];
        $row['categories']=$row['categories'] ? explode(', ',$row['categories']) : [];
        $row['cover_path']=$row['local_cover_path']?:($row['cover_url']?:($row['cover_path']?:'/assets/placeholders/cover-generic.svg'));
        $row['banner_path']=$row['local_banner_path']?:($row['banner_url']?:($row['banner_path']?:'/assets/placeholders/banner-generic.svg'));
        $row['icon_path']=$row['local_icon_path']?:($row['icon_url']?:($row['icon_path']?:'/assets/placeholders/icon-generic.svg'));
        if (isset($row['size_bytes'])) $row['size_bytes']=(int)$row['size_bytes'];
        unset($row['created_by']);
        return $row;
    }
}
