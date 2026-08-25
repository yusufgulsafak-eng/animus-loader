import {api} from "../api/client";
import type {AdminAnnouncement,AdminBanner,AdminCategory,AdminGame,AdminLoaderVersion,AdminPanelData,AdminSubscription,AdminUser,DeletionReport,PatchAction,PatchBuilderData,StorageStatus} from "./types";

type Notice=(message:string,bad?:boolean)=>void;
const escapeHtml=(value:unknown)=>String(value??"").replace(/[&<>"']/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[char]!));
const selected=(value:unknown,current:unknown)=>String(value)===String(current)?" selected":"";
const checked=(value:unknown)=>Boolean(Number(value))?" checked":"";
const dt=(value?:string|null)=>value?String(value).replace(" ","T").slice(0,16):"";
const actionTypes=["COPY_FILE","COPY_DIRECTORY","REPLACE_FILE","DELETE_FILE","DELETE_DIRECTORY","CREATE_DIRECTORY","MOVE_FILE","RENAME_FILE"];

type Deleter=(entity:string,id:number,label:string)=>Promise<void>;

/** Varlık türü -> sunucu action'ı ve id alanı. Tek yerde tanımlı. */
const DELETE_ACTIONS:Record<string,{action:string;key:string}>={
  game:           {action:"delete_game",           key:"game_id"},
  patch_version:  {action:"delete_patch_version",  key:"version_id"},
  loader_version: {action:"delete_loader_version", key:"id"},
  category:       {action:"delete_category",       key:"id"},
  announcement:   {action:"delete_announcement",   key:"id"},
  banner:         {action:"delete_banner",         key:"id"},
  subscription:   {action:"delete_subscription",   key:"id"},
  user:           {action:"delete_user",           key:"id"}
};
/** Sunucudan silme öncesi etki raporu alınabilen varlıklar. */
const DESCRIBABLE=new Set(["game","patch_version","loader_version","category","user"]);

export const humanBytes=(value:unknown):string=>{
  const n=Number(value||0);
  if(!n)return "0 B";
  const units=["B","KB","MB","GB","TB"];
  const index=Math.min(units.length-1,Math.floor(Math.log(n)/Math.log(1024)));
  return (n/Math.pow(1024,index)).toFixed(index?1:0)+" "+units[index];
};

/** Etki raporunu kullanıcının okuyabileceği onay metnine çevirir. */
export function impactText(report:DeletionReport):string{
  const lines:string[]=[];
  (report.blocking||[]).forEach(item=>lines.push("! "+item));
  const cascade=(report.cascade||{}) as Record<string,any>;
  if(cascade.patch_versions!==undefined)lines.push(`Silinecek yama sürümü: ${cascade.patch_versions} (yayında: ${cascade.published_versions||0})`);
  if(cascade.archive_bytes)lines.push("Karantinaya alınacak arşiv: "+humanBytes(cascade.archive_bytes));
  if(cascade.package_bytes)lines.push("Karantinaya alınacak paket: "+humanBytes(cascade.package_bytes));
  if(cascade.download_logs_detached)lines.push("İndirme kaydı korunacak (anonimleşir): "+cascade.download_logs_detached);
  if(cascade.games_unlinked)lines.push("Kategorisi kaldırılacak oyun: "+cascade.games_unlinked);
  if(cascade.replacement_version)lines.push("Kanal şu sürüme geri dönecek: "+cascade.replacement_version);
  if(cascade.is_active_release&&!cascade.replacement_version)lines.push("Bu kanalda yayında yama kalmayacak.");
  if(cascade.subscriptions)lines.push("Silinecek abonelik: "+cascade.subscriptions);
  if(cascade.note)lines.push(String(cascade.note));
  return lines.join("\n");
}

/**
 * Kalıcı silme akışı: önce sunucudan gerçek etki raporu alınır, kullanıcı
 * neyi kaybedeceğini görerek onaylar; yayında olan kayıtlar ikinci bir onay ister.
 */
async function requestDelete(entity:string,id:number,label:string,notify:Notice):Promise<boolean>{
  let force=false;
  let detail="";
  if(DESCRIBABLE.has(entity)){
    try{
      const report=await api.adminAction<DeletionReport>("describe_deletion",{entity,id});
      force=Boolean(report.requires_force);
      const text=impactText(report);
      if(text)detail="\n\n"+text;
    }catch(error){notify(error instanceof Error?error.message:"Silme raporu alınamadı.",true);return false}
  }
  if(!confirm(`KALICI SİLME\n\n"${label}" kalıcı olarak silinecek.${detail}\n\nDevam edilsin mi?`))return false;
  if(force&&!confirm("Bu kayıt yayında veya bağımlılığı var.\nYine de silmek istediğinize emin misiniz?"))return false;
  const map=DELETE_ACTIONS[entity];
  if(!map)throw new Error("Bu tür için silme işlemi tanımlı değil: "+entity);
  await api.adminAction(map.action,{[map.key]:id,force});
  return true;
}

export async function renderAdminPanel(host:HTMLElement,onChanged:()=>Promise<void>,notify:Notice):Promise<void>{
  host.innerHTML='<section class="loader-admin admin-loading"><div class="spinner"></div><h2>Yönetim verileri yükleniyor…</h2></section>';
  try{
    let data=await api.adminPanel();
    const reload=async(refresh=false)=>{data=await api.adminPanel();draw();if(refresh)await onChanged()};
    const run=async(task:()=>Promise<unknown>,message:string,refresh=false)=>{try{await task();notify(message);await reload(refresh)}catch(error){notify(error instanceof Error?error.message:"İşlem tamamlanamadı.",true)}};
    const del:Deleter=async(entity,id,label)=>{
      try{
        if(!await requestDelete(entity,id,label,notify))return;
        notify("Silindi: "+label);
        await reload(true);
      }catch(error){notify(error instanceof Error?error.message:"Silme tamamlanamadı.",true)}
    };
    const draw=()=>{
      host.innerHTML=layout(data);
      bindTabs(host);
      bindGames(host,data,run,del);
      bindPatches(host,data,run,notify,del);
      bindCategories(host,data,run,del);
      bindAnnouncements(host,data,run,del);
      bindBanners(host,run,del);
      bindUsers(host,data,run,del);
      bindSubscriptions(host,data,run,del);
      bindLoaderVersions(host,del);
      bindMaintenance(host,notify);
    };
    draw();
  }catch(error){
    host.innerHTML=`<section class="loader-admin admin-denied"><h2>Yönetim paneli açılamadı</h2><p>${escapeHtml(error instanceof Error?error.message:"Bu işlem için yetkiniz bulunmuyor.")}</p><button id="admin-retry">Tekrar Dene</button></section>`;
    host.querySelector("#admin-retry")?.addEventListener("click",()=>void renderAdminPanel(host,onChanged,notify));
  }
}

function layout(data:AdminPanelData):string{
  const s=data.stats;
  return `<section class="loader-admin">
    <div class="admin-heading"><div><span class="overline">YALNIZCA YETKİLİ HESAPLAR</span><h1>Yönetim Paneli</h1><p>Animus kataloğu ve yayın akışını loader içinden yönet.</p></div><div class="admin-statline"><b>${s.games}</b> oyun · <b>${s.patches}</b> sürüm · <b>${s.downloads}</b> indirme</div></div>
    <div class="admin-tabs">
      <button class="active" data-admin-tab="games">Oyunlar</button><button data-admin-tab="patches">Yamalar</button><button data-admin-tab="categories">Kategoriler</button><button data-admin-tab="announcements">Duyurular</button><button data-admin-tab="banners">Bannerlar</button><button data-admin-tab="users">Kullanıcılar</button><button data-admin-tab="subscriptions">Abonelikler</button><button data-admin-tab="loader">Loader Sürümleri</button><button data-admin-tab="maintenance">Bakım</button>
    </div>
    <div class="admin-panel active" data-admin-panel="games">${gamesView(data)}</div>
    <div class="admin-panel" data-admin-panel="patches">${patchesView(data)}</div>
    <div class="admin-panel" data-admin-panel="categories">${categoriesView(data.categories)}</div>
    <div class="admin-panel" data-admin-panel="announcements">${announcementsView(data.announcements)}</div>
    <div class="admin-panel" data-admin-panel="banners">${bannersView(data.banners)}</div>
    <div class="admin-panel" data-admin-panel="users">${usersView(data.users||[])}</div>
    <div class="admin-panel" data-admin-panel="subscriptions">${subscriptionsView(data)}</div>
    <div class="admin-panel" data-admin-panel="loader">${loaderVersionsView(data.loader_versions||[])}</div>
    <div class="admin-panel" data-admin-panel="maintenance">${maintenanceView()}</div>
  </section>`;
}

function gamesView(data:AdminPanelData):string{
  const categories=data.categories.map(category=>`<label><input type="checkbox" name="category_ids" value="${category.id}"> ${escapeHtml(category.name)}</label>`).join("");
  const rows=data.games.map(game=>`<tr><td><b>${escapeHtml(game.name)}</b><small>${escapeHtml(game.slug)}</small></td><td>${game.access_type==="premium"?"Premium":"Ücretsiz"}</td><td>%${game.translation_percent}</td><td>${Number(game.is_active)?"Aktif":"Pasif"}</td><td class="admin-actions"><button data-game-edit="${game.id}">Düzenle</button><button data-game-toggle="${game.id}" data-active="${Number(game.is_active)?0:1}">${Number(game.is_active)?"Pasife Al":"Aktifleştir"}</button><button class="danger" data-game-delete="${game.id}">Sil</button></td></tr>`).join("");
  return `<div class="admin-grid two">
    <form id="admin-game-form" class="admin-card"><div class="admin-card-title"><h2>Oyun Ekle / Düzenle</h2><button type="button" id="admin-new-game">Temizle</button></div>
      <input type="hidden" name="id" value="0"><label>Oyun adı<input name="name" required maxlength="190"></label><label>Slug<input name="slug" required pattern="[a-z0-9]+(?:-[a-z0-9]+)*"></label>
      <label>Kısa açıklama<textarea name="short_description" maxlength="500"></textarea></label><label>Açıklama<textarea name="description" rows="4"></textarea></label><div class="admin-form-grid"><label>Harici kapak URL<input name="cover_url" type="url" placeholder="https://..."></label><label>Harici oyun banner URL<input name="banner_url" type="url" placeholder="https://..."></label></div>
      <div class="admin-form-grid"><label>Steam App ID<input name="steam_app_id"></label><label>Epic ID<input name="epic_catalog_id"></label><label>Executable<input name="executable"></label><label>Process name<input name="process_name"></label>
      <label>Erişim<select name="access_type"><option value="free">Ücretsiz</option><option value="premium">Premium</option></select></label><label>Çeviri %<input name="translation_percent" type="number" min="0" max="100" value="0"></label><label>Minimum loader<input name="minimum_loader_version" value="0.1.0"></label><label>Mağazalar<input name="supported_stores" value="manual" placeholder="steam, epic, manual"></label></div>
      <fieldset><legend>Kategoriler</legend><div class="admin-checks">${categories||"<small>Önce kategori oluşturun.</small>"}</div></fieldset>
      <label>Zorunlu dosyalar<textarea name="required_files" rows="3" placeholder="Her satıra bir relative path"></textarea></label><label>Opsiyonel dosyalar<textarea name="optional_files" rows="2"></textarea></label>
      <label class="admin-check"><input name="is_active" type="checkbox"> Aktif</label><button type="submit" class="admin-primary">Oyunu Kaydet</button>
    </form>
    <div class="admin-stack"><form id="admin-media-form" class="admin-card"><h2>Oyun Görselleri</h2><p class="admin-help">Kapak kartlarda, oyun bannerı ise oyun detay ekranının üst bölümünde görünür.</p><label>Oyun<select name="game_id" required><option value="">Seçin</option>${gameOptions(data.games)}</select></label><label>Görsel türü<select name="kind"><option value="cover">Kapak</option><option value="banner">Oyun Bannerı</option><option value="icon">İkon</option></select></label><label>JPG / PNG / WebP<input name="image" type="file" accept="image/jpeg,image/png,image/webp" required></label><button class="admin-primary">Yükle / Değiştir</button><button type="button" class="danger" id="admin-media-delete">Seçili Görseli Kaldır</button></form>
      <div class="admin-card admin-note"><b>Güvenli silme</b><p>Sil işlemi oyunla ilişkili patch metadata ve private arşivleri de kaldırır. Yanlışlıkla silmeyi önlemek için ayrıca onay istenir.</p></div></div>
    </div>
    <div class="admin-table-wrap"><table><thead><tr><th>Oyun</th><th>Erişim</th><th>Çeviri</th><th>Durum</th><th>İşlem</th></tr></thead><tbody>${rows||'<tr><td colspan="5">Oyun yok.</td></tr>'}</tbody></table></div>`;
}

function patchesView(data:AdminPanelData):string{
  const rows=data.versions.map(version=>`<tr><td><b>${escapeHtml(version.game_name)}</b><small>${version.source_type==="external"?"Harici link · ":"Sunucu · "}${escapeHtml(version.original_name||"Arşiv yok")}</small></td><td>${escapeHtml(version.version)}</td><td>${escapeHtml(version.channel)}</td><td>${escapeHtml(version.status)}</td><td class="admin-actions"><button data-builder="${version.id}">Builder</button><button data-publish="${version.id}">Yayınla</button>${version.status==="ARCHIVED"?`<button data-rollback="${version.id}">Rollback</button>`:""}<select data-patch-status="${version.id}"><option value="">Durum…</option><option>TESTING</option><option>DISABLED</option><option>ARCHIVED</option><option>DRAFT</option></select><button class="danger" data-version-delete="${version.id}" data-label="${escapeHtml(version.game_name+" "+version.version+" ("+version.channel+")")}">Sil</button></td></tr>`).join("");
  return `<div class="admin-grid two"><form id="admin-patch-form" class="admin-card" enctype="multipart/form-data"><h2>Yeni Patch Sürümü</h2><label>Oyun<select name="game_id" required><option value="">Seçin</option>${gameOptions(data.games)}</select></label><div class="admin-form-grid"><label>Patch version<input name="version" required placeholder="1.0.0"></label><label>Oyun version<input name="game_version"></label><label>Kanal<select name="channel"><option>stable</option><option>beta</option><option selected>internal</option></select></label><label>Erişim<select name="access_type"><option value="free">Ücretsiz</option><option value="premium">Premium</option></select></label><label>Minimum loader<input name="minimum_loader_version" value="0.1.0"></label><label class="admin-check"><input type="checkbox" name="mandatory_update" value="1"> Zorunlu güncelleme</label></div><label>Changelog<textarea name="changelog" rows="4"></textarea></label><label>Patch Kaynağı<select name="source_type" id="patch-source-type"><option value="server">Sunucuya ZIP Yükle</option><option value="external">Harici İndirme Linki (MediaFire / Drive / CDN)</option></select></label><div id="patch-server-source"><label>Patch ZIP<input name="archive" type="file" accept=".zip,application/zip"></label></div><div id="patch-external-source" hidden><label>İndirme / paylaşım URL<input name="external_url" type="url" placeholder="https://pixeldrain.com/u/..."></label><div class="admin-inline"><button type="button" id="patch-read-link">Linki Oku / Otomatik Doldur</button><small>PixelDrain paylaşım linkini doğrudan yapıştırabilirsiniz.</small></div><label>SHA-256<input name="sha256" pattern="[A-Fa-f0-9]{64}" placeholder="PixelDrain için otomatik"></label><label>Dosya boyutu (byte)<input name="size_bytes" type="number" min="1" placeholder="PixelDrain için otomatik"></label><label>Dosya adı<input name="original_name" placeholder="PixelDrain için otomatik"></label><small>PixelDrain: paylaşım linki yeterlidir; direct URL, SHA-256, boyut ve dosya adı otomatik alınır. Diğer sağlayıcılarda direct HTTPS URL + SHA-256 + boyut girin.</small></div><button class="admin-primary">Draft Sürüm Oluştur</button></form>
    <section class="admin-card" id="patch-builder"><div class="admin-card-title"><h2>Patch Builder</h2><button id="builder-add-action" type="button">Action Ekle</button></div><label>Version ID<input id="builder-version-id" type="number" min="1"></label><button type="button" id="builder-load">Arşiv ve Actionları Yükle</button><div id="builder-tree" class="admin-file-tree"></div><div id="builder-actions"></div><div class="admin-actions wide"><button id="builder-save" type="button">Actionları Kaydet</button><button id="builder-test" type="button">Manifest Test</button></div><pre id="builder-output"></pre></section></div>
    <div class="admin-table-wrap"><table><thead><tr><th>Oyun</th><th>Sürüm</th><th>Kanal</th><th>Durum</th><th>İşlem</th></tr></thead><tbody>${rows||'<tr><td colspan="5">Patch sürümü yok.</td></tr>'}</tbody></table></div>`;
}

function categoriesView(categories:AdminCategory[]):string{
  return `<div class="admin-grid two"><form id="admin-category-form" class="admin-card"><h2>Kategori</h2><input type="hidden" name="id" value="0"><label>Ad<input name="name" required></label><label>Slug<input name="slug" required></label><label>Sıra<input name="sort_order" type="number" value="0"></label><label class="admin-check"><input name="is_active" type="checkbox" checked> Aktif</label><button class="admin-primary">Kaydet</button></form><div class="admin-card admin-list">${categories.map(item=>`<article><div><b>${escapeHtml(item.name)}</b><small>${escapeHtml(item.slug)}</small></div><button data-category-edit="${item.id}">Düzenle</button><button class="danger" data-category-delete="${item.id}">Sil</button></article>`).join("")||"<p>Kategori yok.</p>"}</div></div>`;
}

function announcementsView(items:AdminAnnouncement[]):string{
  return `<div class="admin-grid two"><form id="admin-announcement-form" class="admin-card"><h2>Duyuru</h2><input type="hidden" name="id" value="0"><label>Başlık<input name="title" required></label><label>Metin<textarea name="body" rows="5" required></textarea></label><label>Hedef<select name="audience"><option>all</option><option>free</option><option>premium</option><option>tester</option><option>admin</option></select></label><div class="admin-form-grid"><label>Başlangıç<input name="starts_at" type="datetime-local"></label><label>Bitiş<input name="ends_at" type="datetime-local"></label></div><label class="admin-check"><input name="is_active" type="checkbox" checked> Aktif</label><button class="admin-primary">Kaydet</button></form><div class="admin-card admin-list">${items.map(item=>`<article><div><b>${escapeHtml(item.title)}</b><small>${escapeHtml(item.audience)} · ${Number(item.is_active)?"Aktif":"Pasif"}</small><p>${escapeHtml(item.body)}</p></div><button data-announcement-edit="${item.id}">Düzenle</button><button class="danger" data-announcement-delete="${item.id}">Sil</button></article>`).join("")||"<p>Duyuru yok.</p>"}</div></div>`;
}

function bannersView(items:AdminBanner[]):string{
  return `<div class="admin-grid two"><form id="admin-banner-form" class="admin-card" enctype="multipart/form-data"><h2>Yeni Banner</h2><label>Başlık<input name="title" required></label><label>Hedef URL<input name="target_url"></label><label>Sıra<input name="sort_order" type="number" value="0"></label><label>JPG / PNG / WebP<input name="image" type="file" accept="image/jpeg,image/png,image/webp" required></label><label class="admin-check"><input name="is_active" type="checkbox" value="1" checked> Aktif</label><button class="admin-primary">Banner Yükle</button></form><div class="admin-card admin-list">${items.map(item=>`<article><div><b>${escapeHtml(item.title)}</b><small>${escapeHtml(item.image_path)}</small></div><button class="danger" data-banner-delete="${item.id}" data-label="${escapeHtml(item.title)}">Sil</button></article>`).join("")||"<p>Banner yok.</p>"}</div></div>`;
}

function gameOptions(games:AdminGame[]):string{return games.map(game=>`<option value="${game.id}">${escapeHtml(game.name)}</option>`).join("")}
function bindTabs(host:HTMLElement){host.querySelectorAll<HTMLButtonElement>("[data-admin-tab]").forEach(button=>button.onclick=()=>{host.querySelectorAll("[data-admin-tab]").forEach(x=>x.classList.remove("active"));host.querySelectorAll("[data-admin-panel]").forEach(x=>x.classList.remove("active"));button.classList.add("active");host.querySelector(`[data-admin-panel="${button.dataset.adminTab}"]`)?.classList.add("active")})}
function lines(value:FormDataEntryValue|null){return String(value||"").split(/\r?\n/).map(x=>x.trim()).filter(Boolean)}

function bindGames(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-game-form")!;
  host.querySelector("#admin-new-game")!.addEventListener("click",()=>{form.reset();(form.elements.namedItem("id") as HTMLInputElement).value="0"});
  host.querySelectorAll<HTMLButtonElement>("[data-game-edit]").forEach(button=>button.onclick=()=>{const game=data.games.find(x=>x.id===Number(button.dataset.gameEdit));if(!game)return;for(const [key,value] of Object.entries(game)){const field=form.elements.namedItem(key);if(!field||key==="category_ids")continue;if(field instanceof HTMLInputElement&&field.type==="checkbox")field.checked=Boolean(Number(value));else if(field instanceof HTMLInputElement||field instanceof HTMLTextAreaElement||field instanceof HTMLSelectElement)field.value=Array.isArray(value)?value.join(", "):String(value??"")}form.querySelectorAll<HTMLInputElement>('input[name="category_ids"]').forEach(x=>x.checked=game.category_ids.includes(Number(x.value)));form.scrollIntoView({behavior:"smooth"})});
  form.onsubmit=event=>{event.preventDefault();const values=new FormData(form);const game={id:Number(values.get("id")||0),name:String(values.get("name")||""),slug:String(values.get("slug")||""),short_description:String(values.get("short_description")||""),description:String(values.get("description")||""),cover_url:String(values.get("cover_url")||""),banner_url:String(values.get("banner_url")||""),steam_app_id:String(values.get("steam_app_id")||""),epic_catalog_id:String(values.get("epic_catalog_id")||""),executable:String(values.get("executable")||""),process_name:String(values.get("process_name")||""),access_type:String(values.get("access_type")||"free"),translation_percent:Number(values.get("translation_percent")||0),minimum_loader_version:String(values.get("minimum_loader_version")||"0.1.0"),supported_stores:String(values.get("supported_stores")||"manual").split(",").map(x=>x.trim()).filter(Boolean),required_files:lines(values.get("required_files")),optional_files:lines(values.get("optional_files")),category_ids:values.getAll("category_ids").map(Number),is_active:values.has("is_active")};void run(()=>api.adminAction("save_game",{game}),"Oyun kaydedildi.",true)};
  host.querySelectorAll<HTMLButtonElement>("[data-game-toggle]").forEach(button=>button.onclick=()=>void run(()=>api.adminAction("set_game_status",{game_id:Number(button.dataset.gameToggle),active:button.dataset.active==="1"}),"Oyun durumu güncellendi.",true));
  host.querySelectorAll<HTMLButtonElement>("[data-game-delete]").forEach(button=>button.onclick=()=>{const game=data.games.find(x=>x.id===Number(button.dataset.gameDelete));if(game)void del("game",game.id,game.name)});
  const mediaForm=host.querySelector<HTMLFormElement>("#admin-media-form")!;mediaForm.onsubmit=event=>{event.preventDefault();const upload=new FormData(mediaForm);const kind=String(upload.get("kind")||"cover");void run(()=>api.adminUpload("upload_game_image",upload),`${kind==="banner"?"Oyun bannerı":kind==="icon"?"İkon":"Kapak"} güncellendi.`,true)};
  host.querySelector("#admin-media-delete")!.addEventListener("click",()=>{const values=new FormData(mediaForm);const id=Number(values.get("game_id"));const kind=String(values.get("kind")||"cover");if(id&&confirm("Seçili oyun görseli kaldırılsın mı?"))void run(()=>api.adminAction("delete_game_image",{game_id:id,kind}),"Görsel kaldırıldı.",true)});
}

function bindPatches(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,notify:Notice,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-patch-form")!;const sourceType=host.querySelector<HTMLSelectElement>("#patch-source-type")!,serverBox=host.querySelector<HTMLElement>("#patch-server-source")!,externalBox=host.querySelector<HTMLElement>("#patch-external-source")!;const syncSource=()=>{const external=sourceType.value==="external";serverBox.hidden=external;externalBox.hidden=!external;const archive=form.elements.namedItem("archive") as HTMLInputElement;archive.required=!external;const url=form.elements.namedItem("external_url") as HTMLInputElement;url.required=external};sourceType.onchange=syncSource;syncSource();
  const readExternal=async()=>{const url=(form.elements.namedItem("external_url") as HTMLInputElement).value.trim();if(!url){notify("Önce harici linki yapıştırın.",true);return null}try{const meta=await api.adminAction<{direct_url:string;sha256:string;size_bytes:number;original_name:string}>("inspect_external_patch",{url});(form.elements.namedItem("external_url") as HTMLInputElement).value=meta.direct_url;(form.elements.namedItem("sha256") as HTMLInputElement).value=meta.sha256;(form.elements.namedItem("size_bytes") as HTMLInputElement).value=String(meta.size_bytes);(form.elements.namedItem("original_name") as HTMLInputElement).value=meta.original_name;notify("PixelDrain bilgileri otomatik dolduruldu.");return meta}catch(error){notify(error instanceof Error?error.message:"Link bilgisi alınamadı.",true);return null}};
  host.querySelector<HTMLButtonElement>("#patch-read-link")!.onclick=()=>void readExternal();
  form.onsubmit=async event=>{event.preventDefault();let payload=new FormData(form);const external=String(payload.get("source_type"))==="external";if(external){const hash=String(payload.get("sha256")||"");const bytes=Number(payload.get("size_bytes")||0);if(!/^[A-Fa-f0-9]{64}$/.test(hash)||bytes<1){const meta=await readExternal();if(!meta)return;payload=new FormData(form)}}void run(()=>api.adminUpload("create_patch",payload),external?"Harici patch kaynağı kaydedildi ve draft sürüm oluşturuldu.":"Patch ZIP yüklendi ve draft sürüm oluşturuldu.",true)};
  host.querySelectorAll<HTMLButtonElement>("[data-publish]").forEach(button=>button.onclick=()=>{if(confirm("Manifest kontrolleri başarılıysa bu sürüm yayınlansın mı?"))void run(()=>api.adminAction("publish_patch",{version_id:Number(button.dataset.publish)}),"Patch yayınlandı.",true)});
  host.querySelectorAll<HTMLButtonElement>("[data-rollback]").forEach(button=>button.onclick=()=>{if(confirm("Bu arşivlenmiş sürüm yeniden aktif yayın olsun mu?"))void run(()=>api.adminAction("rollback_patch",{version_id:Number(button.dataset.rollback)}),"Rollback tamamlandı.",true)});
  host.querySelectorAll<HTMLSelectElement>("[data-patch-status]").forEach(select=>select.onchange=()=>{if(select.value)void run(()=>api.adminAction("set_patch_status",{version_id:Number(select.dataset.patchStatus),status:select.value}),"Patch durumu güncellendi.",true)});
  host.querySelectorAll<HTMLButtonElement>("[data-version-delete]").forEach(button=>button.onclick=()=>void del("patch_version",Number(button.dataset.versionDelete),button.dataset.label||"Yama sürümü"));
  let builder:PatchBuilderData|null=null;const idInput=host.querySelector<HTMLInputElement>("#builder-version-id")!,list=host.querySelector<HTMLElement>("#builder-actions")!,tree=host.querySelector<HTMLElement>("#builder-tree")!,output=host.querySelector<HTMLElement>("#builder-output")!;
  const add=(action:Partial<PatchAction>={})=>{const row=document.createElement("div");row.className="builder-action";row.dataset.id=action.id||crypto.randomUUID();row.innerHTML=`<select class="builder-type">${actionTypes.map(type=>`<option${selected(type,action.type||"COPY_FILE")}>${type}</option>`).join("")}</select><input class="builder-source" placeholder="ZIP kaynak yolu" value="${escapeHtml(action.source||"")}"><input class="builder-destination" placeholder="Game root relative hedef" value="${escapeHtml(action.destination||"")}"><label><input class="builder-backup" type="checkbox"${action.backup===false?"":" checked"}> Backup</label><button type="button" class="danger">Sil</button>`;row.querySelector("button")!.onclick=()=>row.remove();list.append(row)};
  const load=async(id:number)=>{try{builder=await api.adminAction<PatchBuilderData>("load_patch_builder",{version_id:id});idInput.value=String(id);tree.innerHTML=builder.file_tree.length?`<b>ZIP içeriği</b>${builder.file_tree.map(x=>`<code>${escapeHtml(x.path)}</code>`).join("")}`:"<small>Arşiv içeriği boş.</small>";list.innerHTML="";builder.actions.forEach(add);notify("Patch Builder yüklendi.")}catch(error){notify(error instanceof Error?error.message:"Builder yüklenemedi.",true)}};
  host.querySelectorAll<HTMLButtonElement>("[data-builder]").forEach(button=>button.onclick=()=>void load(Number(button.dataset.builder)));host.querySelector("#builder-load")!.addEventListener("click",()=>void load(Number(idInput.value)));host.querySelector("#builder-add-action")!.addEventListener("click",()=>add());
  const actions=()=>[...list.querySelectorAll<HTMLElement>(".builder-action")].map(row=>({id:row.dataset.id!,type:(row.querySelector(".builder-type") as HTMLSelectElement).value,source:(row.querySelector(".builder-source") as HTMLInputElement).value||null,destination:(row.querySelector(".builder-destination") as HTMLInputElement).value,backup:(row.querySelector(".builder-backup") as HTMLInputElement).checked}));
  host.querySelector("#builder-save")!.addEventListener("click",()=>void run(()=>api.adminAction("save_actions",{version_id:Number(idInput.value),actions:actions()}),"Patch actionları kaydedildi."));
  host.querySelector("#builder-test")!.addEventListener("click",async()=>{try{const result=await api.adminAction("test_manifest",{version_id:Number(idInput.value)});output.textContent=JSON.stringify(result,null,2);notify("Manifest testi tamamlandı.")}catch(error){notify(error instanceof Error?error.message:"Manifest testi başarısız.",true)}});
}

function bindCategories(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-category-form")!;form.onsubmit=event=>{event.preventDefault();const values=new FormData(form);const category={id:Number(values.get("id")||0),name:String(values.get("name")||""),slug:String(values.get("slug")||""),sort_order:Number(values.get("sort_order")||0),is_active:values.has("is_active")};void run(()=>api.adminAction("save_category",{category}),"Kategori kaydedildi.",true)};
  host.querySelectorAll<HTMLButtonElement>("[data-category-edit]").forEach(button=>button.onclick=()=>fillForm(form,data.categories.find(x=>x.id===Number(button.dataset.categoryEdit))));
  host.querySelectorAll<HTMLButtonElement>("[data-category-delete]").forEach(button=>button.onclick=()=>{const id=Number(button.dataset.categoryDelete);const category=data.categories.find(x=>x.id===id);void del("category",id,category?.name||"Kategori")});
}
function bindAnnouncements(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-announcement-form")!;form.onsubmit=event=>{event.preventDefault();const values=new FormData(form);const announcement={id:Number(values.get("id")||0),title:String(values.get("title")||""),body:String(values.get("body")||""),audience:String(values.get("audience")||"all"),starts_at:String(values.get("starts_at")||""),ends_at:String(values.get("ends_at")||""),is_active:values.has("is_active")};void run(()=>api.adminAction("save_announcement",{announcement}),"Duyuru kaydedildi.",true)};
  host.querySelectorAll<HTMLButtonElement>("[data-announcement-edit]").forEach(button=>button.onclick=()=>fillForm(form,data.announcements.find(x=>x.id===Number(button.dataset.announcementEdit))));
  host.querySelectorAll<HTMLButtonElement>("[data-announcement-delete]").forEach(button=>button.onclick=()=>{const id=Number(button.dataset.announcementDelete);const item=data.announcements.find(x=>x.id===id);void del("announcement",id,item?.title||"Duyuru")});
}
function bindBanners(host:HTMLElement,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-banner-form")!;form.onsubmit=event=>{event.preventDefault();void run(()=>api.adminUpload("save_banner",new FormData(form)),"Banner yüklendi.",true)};
  host.querySelectorAll<HTMLButtonElement>("[data-banner-delete]").forEach(button=>button.onclick=()=>{const id=Number(button.dataset.bannerDelete);void del("banner",id,button.dataset.label||"Banner")});
}
function fillForm(form:HTMLFormElement,item:AdminCategory|AdminAnnouncement|undefined){if(!item)return;for(const [key,value] of Object.entries(item)){const field=form.elements.namedItem(key);if(field instanceof HTMLInputElement&&field.type==="checkbox")field.checked=Boolean(Number(value));else if(field instanceof HTMLInputElement||field instanceof HTMLTextAreaElement||field instanceof HTMLSelectElement)field.value=field.type==="datetime-local"?dt(String(value||"")):String(value??"")}form.scrollIntoView({behavior:"smooth"})}

// ---------------------------------------------------------------------------
// Kullanıcılar / Abonelikler / Loader sürümleri / Bakım
// Bu bölümler daha önce yalnızca web panelinde vardı; loader içindeki yönetim
// ekranı bunlara hiç erişemiyordu.
// ---------------------------------------------------------------------------

function usersView(users:AdminUser[]):string{
  const roles=["user","tester","admin","super_admin"];
  const channels=["stable","beta","internal"];
  const statuses=["active","suspended","pending"];
  const rows=users.map(user=>`<tr data-user-row="${user.id}">
    <td><b>${escapeHtml(user.display_name)}</b><small>${escapeHtml(user.email)}</small></td>
    <td><select class="user-role">${roles.map(role=>`<option${selected(role,user.role)}>${role}</option>`).join("")}</select></td>
    <td><select class="user-channel">${channels.map(channel=>`<option${selected(channel,user.release_channel)}>${channel}</option>`).join("")}</select></td>
    <td><select class="user-status">${statuses.map(status=>`<option${selected(status,user.status)}>${status}</option>`).join("")}</select></td>
    <td class="admin-actions"><button data-user-save="${user.id}">Kaydet</button><button class="danger" data-user-delete="${user.id}" data-label="${escapeHtml(user.email)}">Sil</button></td>
  </tr>`).join("");
  return `<div class="admin-card admin-note"><b>Kullanıcı silme</b><p>Kalıcı silme yalnız süper adminlere açıktır. Abonelikler ve oturum anahtarları birlikte silinir; indirme geçmişi anonimleştirilerek korunur.</p></div>
    <div class="admin-table-wrap"><table><thead><tr><th>Kullanıcı</th><th>Rol</th><th>Kanal</th><th>Durum</th><th>İşlem</th></tr></thead><tbody>${rows||'<tr><td colspan="5">Kullanıcı yok.</td></tr>'}</tbody></table></div>`;
}

function subscriptionsView(data:AdminPanelData):string{
  const users=data.users||[];
  const subscriptions=data.subscriptions||[];
  const rows=subscriptions.map(item=>`<tr>
    <td><b>${escapeHtml(item.display_name)}</b><small>${escapeHtml(item.email)}</small></td>
    <td>${escapeHtml(item.plan_name)}</td><td>${escapeHtml(item.status)}</td>
    <td>${escapeHtml(item.ends_at||"Süresiz")}</td>
    <td class="admin-actions"><button data-subscription-toggle="${item.id}" data-status="${item.status==="active"?"cancelled":"active"}">${item.status==="active"?"İptal Et":"Aktifleştir"}</button><button class="danger" data-subscription-delete="${item.id}" data-label="${escapeHtml(item.display_name+" · "+item.plan_name)}">Sil</button></td>
  </tr>`).join("");
  return `<div class="admin-grid two"><form id="admin-subscription-form" class="admin-card"><h2>Manuel Abonelik</h2>
    <label>Kullanıcı<select name="user_id" required><option value="">Seçin</option>${users.map(user=>`<option value="${user.id}">${escapeHtml(user.email)}</option>`).join("")}</select></label>
    <label>Plan adı<input name="plan_name" required placeholder="Premium"></label>
    <label>Durum<select name="status"><option>active</option><option>trial</option><option>expired</option><option>cancelled</option></select></label>
    <div class="admin-form-grid"><label>Başlangıç<input name="starts_at" type="datetime-local"></label><label>Bitiş<input name="ends_at" type="datetime-local"></label></div>
    <button class="admin-primary">Abonelik Tanımla</button></form>
    <div class="admin-card admin-note"><b>Ödeme yok</b><p>Bu bölüm ödeme üretmez; yalnızca yetkili adminin manuel erişim tanımıdır.</p></div></div>
    <div class="admin-table-wrap"><table><thead><tr><th>Kullanıcı</th><th>Plan</th><th>Durum</th><th>Bitiş</th><th>İşlem</th></tr></thead><tbody>${rows||'<tr><td colspan="5">Abonelik yok.</td></tr>'}</tbody></table></div>`;
}

function loaderVersionsView(versions:AdminLoaderVersion[]):string{
  const rows=versions.map(version=>`<tr>
    <td><b>${escapeHtml(version.version)}</b><small>${escapeHtml(String(version.sha256||"").slice(0,16))}…</small></td>
    <td>${escapeHtml(version.channel)}</td><td>${humanBytes(version.size_bytes)}</td>
    <td>${Number(version.mandatory)?"Zorunlu":"Opsiyonel"}</td><td>${escapeHtml(version.published_at)}</td>
    <td class="admin-actions"><button class="danger" data-loader-delete="${version.id}" data-label="${escapeHtml(version.version+" ("+version.channel+")")}">Sil</button></td>
  </tr>`).join("");
  return `<div class="admin-card admin-note"><b>Loader paketleri</b><p>Yeni paket yükleme web panelindeki "Loader Sürümleri" bölümünden yapılır. Buradan yayından kaldırma ve silme yapabilirsiniz. Bir kanalın en güncel paketi silinirken ek onay istenir.</p></div>
    <div class="admin-table-wrap"><table><thead><tr><th>Sürüm</th><th>Kanal</th><th>Boyut</th><th>Tip</th><th>Yayın</th><th>İşlem</th></tr></thead><tbody>${rows||'<tr><td colspan="6">Loader sürümü yok.</td></tr>'}</tbody></table></div>`;
}

function maintenanceView():string{
  return `<div class="admin-grid two">
    <div class="admin-card"><h2>Depolama</h2><p class="admin-help">Silinen dosyalar önce sunucudaki karantinaya alınır; kalıcı temizlik buradan yapılır.</p>
      <div class="admin-stats" id="storage-stats"><span>Durum yükleniyor…</span></div>
      <div class="admin-actions wide"><button data-maintenance="storage_status">Durumu Yenile</button><button data-maintenance="run_storage_gc">Bekleyen Silmeleri Dene</button><button data-maintenance="scan_orphans">Orphan Tara</button><button class="danger" data-maintenance="purge_orphans" data-confirm="Veritabanında karşılığı olmayan dosyalar karantinaya alınsın mı?">Orphan Karantinaya Al</button><button class="danger" data-maintenance="purge_trash" data-confirm="Süresi dolmuş karantina dosyaları KALICI silinsin mi?">Karantinayı Temizle</button></div>
    </div>
    <div class="admin-card"><h2>Kayıt Temizliği</h2>
      <label>İndirme logu saklama (gün)<input id="maintenance-log-days" type="number" min="1" max="3650" value="90"></label>
      <label>Terk edilmiş draft (gün)<input id="maintenance-draft-days" type="number" min="1" max="3650" value="30"></label>
      <div class="admin-actions wide"><button data-maintenance="purge_expired_tokens">Süresi Dolmuş Tokenlar</button><button class="danger" data-maintenance="prune_download_logs" data-days="maintenance-log-days" data-confirm="Belirtilen günden eski indirme logları silinsin mi?">Eski Logları Sil</button><button data-maintenance="prune_stale_drafts" data-days="maintenance-draft-days">Draft Adaylarını Listele</button><button class="danger" data-maintenance="prune_stale_drafts" data-days="maintenance-draft-days" data-apply="1" data-confirm="Listelenen eski draft sürümler KALICI silinsin mi?">Eski Draftları Sil</button></div>
    </div></div>
    <pre id="maintenance-output">Bir işlem seçin.</pre>`;
}

function bindUsers(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  host.querySelectorAll<HTMLButtonElement>("[data-user-save]").forEach(button=>button.onclick=()=>{
    const row=button.closest("tr");
    if(!row)return;
    void run(()=>api.adminAction("update_user",{user:{
      id:Number(button.dataset.userSave),
      role:(row.querySelector(".user-role") as HTMLSelectElement).value,
      release_channel:(row.querySelector(".user-channel") as HTMLSelectElement).value,
      status:(row.querySelector(".user-status") as HTMLSelectElement).value
    }}),"Kullanıcı güncellendi.");
  });
  host.querySelectorAll<HTMLButtonElement>("[data-user-delete]").forEach(button=>button.onclick=()=>{
    const id=Number(button.dataset.userDelete);
    const user=(data.users||[]).find(item=>item.id===id);
    void del("user",id,user?.email||button.dataset.label||"Kullanıcı");
  });
}

function bindSubscriptions(host:HTMLElement,data:AdminPanelData,run:(task:()=>Promise<unknown>,message:string,refresh?:boolean)=>Promise<void>,del:Deleter){
  const form=host.querySelector<HTMLFormElement>("#admin-subscription-form");
  if(form)form.onsubmit=event=>{
    event.preventDefault();
    const values=new FormData(form);
    void run(()=>api.adminAction("save_subscription",{subscription:{
      user_id:Number(values.get("user_id")||0),
      plan_name:String(values.get("plan_name")||""),
      status:String(values.get("status")||"active"),
      starts_at:String(values.get("starts_at")||""),
      ends_at:String(values.get("ends_at")||"")
    }}),"Abonelik tanımlandı.",true);
  };
  host.querySelectorAll<HTMLButtonElement>("[data-subscription-toggle]").forEach(button=>button.onclick=()=>
    void run(()=>api.adminAction("set_subscription_status",{id:Number(button.dataset.subscriptionToggle),status:button.dataset.status}),"Abonelik durumu güncellendi.",true));
  host.querySelectorAll<HTMLButtonElement>("[data-subscription-delete]").forEach(button=>button.onclick=()=>{
    const id=Number(button.dataset.subscriptionDelete);
    const item=(data.subscriptions||[]).find(row=>row.id===id);
    void del("subscription",id,item?`${item.display_name} · ${item.plan_name}`:(button.dataset.label||"Abonelik"));
  });
}

function bindLoaderVersions(host:HTMLElement,del:Deleter){
  host.querySelectorAll<HTMLButtonElement>("[data-loader-delete]").forEach(button=>button.onclick=()=>
    void del("loader_version",Number(button.dataset.loaderDelete),button.dataset.label||"Loader sürümü"));
}

function bindMaintenance(host:HTMLElement,notify:Notice){
  const output=host.querySelector<HTMLElement>("#maintenance-output");
  const stats=host.querySelector<HTMLElement>("#storage-stats");
  const renderStatus=(status:StorageStatus)=>{
    if(!stats)return;
    stats.innerHTML=[
      ["Bekleyen silme",String(status.pending)],
      ["Başarısız silme",String(status.failed)],
      ["Karantinadaki dosya",String(status.trash_files)],
      ["Karantina boyutu",humanBytes(status.trash_bytes)]
    ].map(([label,value])=>`<span><small>${label}</small><b>${escapeHtml(value)}</b></span>`).join("");
  };
  const loadStatus=async()=>{try{renderStatus(await api.adminAction<StorageStatus>("storage_status"))}catch{/* panel açılmamış olabilir */}};

  host.querySelectorAll<HTMLButtonElement>("[data-maintenance]").forEach(button=>button.onclick=async()=>{
    if(button.dataset.confirm&&!confirm(button.dataset.confirm))return;
    const payload:Record<string,unknown>={};
    if(button.dataset.days)payload.days=Number(host.querySelector<HTMLInputElement>("#"+button.dataset.days)?.value||0);
    if(button.dataset.apply)payload.apply=true;
    button.disabled=true;
    try{
      const result=await api.adminAction(button.dataset.maintenance!,payload);
      if(output)output.textContent=JSON.stringify(result,null,2);
      notify("İşlem tamamlandı.");
      await loadStatus();
    }catch(error){
      if(output)output.textContent="Hata: "+(error instanceof Error?error.message:"bilinmeyen hata");
      notify(error instanceof Error?error.message:"İşlem tamamlanamadı.",true);
    }finally{button.disabled=false}
  });
  host.querySelector<HTMLButtonElement>('[data-admin-tab="maintenance"]')?.addEventListener("click",()=>void loadStatus());
}
