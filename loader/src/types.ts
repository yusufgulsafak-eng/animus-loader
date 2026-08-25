export type ReleaseChannel = "stable" | "beta" | "internal";
export interface Subscription {plan_name:string;status:"active"|"trial"|"expired"|"cancelled";starts_at:string;ends_at?:string|null}
export interface User {id:number;email:string;display_name:string;role:string;release_channel:ReleaseChannel;premium:boolean;subscription?:Subscription|null;permissions?:Record<string,boolean>}
export interface Game {id:number;name:string;slug:string;short_description?:string|null;description?:string|null;cover_path?:string|null;banner_path?:string|null;icon_path?:string|null;cover_url?:string|null;banner_url?:string|null;icon_url?:string|null;local_cover_path?:string|null;local_banner_path?:string|null;local_icon_path?:string|null;access_type:"free"|"premium";translation_percent:number;patch_version?:string|null;patch_version_id?:number|null;game_version?:string|null;size_bytes?:number|null;categories:string[];steam_app_id?:string|null;epic_catalog_id?:string|null;executable?:string|null;process_name?:string|null;published_at?:string|null;changelog?:string|null;supported_stores?:string[];minimum_loader_version?:string|null}
export type BackgroundType = "default"|"image"|"video";
export interface BackgroundConfig {type:BackgroundType|string;image_url?:string|null;video_url?:string|null;fallback_url?:string|null;overlay?:number;version?:string}
export interface LoaderBranding {login_background?:BackgroundConfig;library_background?:BackgroundConfig}
export interface LoaderConfig {app_name:string;logo_url?:string;banner_url?:string;login_background_url?:string;accent_color:string;library_title:string;support_url?:string;discord_url?:string;youtube_url?:string;instagram_url?:string;x_url?:string;announcements:{id:number;title:string;body:string}[];banners?:{id:number;title:string;image_path:string;target_url?:string}[];branding?:LoaderBranding}
export interface ApiEnvelope<T>{ok:boolean;data:T;error?:{message:string}}
export interface OperationProgress {stage:string;percent:number;message:string;downloaded_bytes?:number;total_bytes?:number;bytes_per_second?:number}
export interface InstallationSummary {game_id:number;game_name:string;patch_id:number;patch_version:string;game_root:string;backup_id:string;created_at:string;root_exists:boolean;backup_exists:boolean;change_count:number}
export interface UninstallReport {restored:number;forced:boolean;conflicts:string[]}
export interface PruneReport {removed_backups:number;freed_bytes:number;cache_bytes:number}
export interface LoaderUpdateInfo {version:string;currentVersion:string;body?:string|null;date?:string|null}
export interface DryRun {created_files:number;changed_files:number;deleted_files:number;backup_files:number;download_bytes:number;estimated_disk_bytes:number;warnings:string[]}
