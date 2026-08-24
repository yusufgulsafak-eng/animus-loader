export interface AdminGame {
  id:number;name:string;slug:string;short_description?:string|null;description?:string|null;
  steam_app_id?:string|null;epic_catalog_id?:string|null;executable?:string|null;process_name?:string|null;
  access_type:"free"|"premium";translation_percent:number;minimum_loader_version:string;
  supported_stores:string[];is_active:number|boolean;required_files:string[];optional_files:string[];
  category_ids:number[];local_cover_path?:string|null;cover_path?:string|null;
}
export interface AdminCategory {id:number;name:string;slug:string;sort_order:number;is_active:number|boolean}
export interface AdminVersion {id:number;game_name:string;version:string;game_version?:string|null;status:string;channel:string;access_type:string;original_name?:string|null;size_bytes?:number|null;created_at:string}
export interface AdminAnnouncement {id:number;title:string;body:string;audience:string;is_active:number|boolean;starts_at?:string|null;ends_at?:string|null}
export interface AdminBanner {id:number;title:string;image_path:string;target_url?:string|null;sort_order:number;is_active:number|boolean}
export interface AdminPanelData {
  stats:{games:number;active_games:number;patches:number;stable:number;beta:number;users:number;today_downloads:number;downloads:number};
  games:AdminGame[];categories:AdminCategory[];versions:AdminVersion[];announcements:AdminAnnouncement[];banners:AdminBanner[];
}
export interface PatchBuilderData {id:number;status:string;version:string;file_tree:{path:string;size:number;directory:boolean}[];actions:PatchAction[]}
export interface PatchAction {id:string;type:string;source?:string|null;destination:string;backup:boolean}
