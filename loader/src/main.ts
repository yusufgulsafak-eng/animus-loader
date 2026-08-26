import "./styles.css";
import "./community.css";
import "./profile.css";
import "./admin/admin.css";
import {invoke} from "@tauri-apps/api/core";
import {api} from "./api/client";
import {WEB_BASE_URL} from "./config";
import {patchService} from "./services/patch";
import {log} from "./services/logger";
import {filterCatalog} from "./catalog";
import {mountBackgroundMedia} from "./background";
import {validateRegistration} from "./auth/validation";
import {cache,installations,state} from "./stores/app";
import {meetsMinimum} from "./services/version";
import {checkForLoaderUpdate} from "./services/selfUpdate";
import {canManage} from "./admin/access";
import {renderAdminPanel} from "./admin/panel";
import type {Game,LoaderConfig,OperationProgress} from "./types";

const app=document.querySelector<HTMLDivElement>("#app")!;
let currentView="home";
let chatPollTimer:number|undefined;
let chatRefreshRunning=false;
const escapeHtml=(value:unknown)=>String(value??"").replace(/[&<>"']/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[char]!));
const asset=(path?:string|null)=>!path?"":(/^https?:\/\//i.test(path)?path:WEB_BASE_URL+(path.startsWith("/")?path:"/"+path));
const cover=(game:Game)=>asset(game.local_cover_path||game.cover_url||game.cover_path)||asset("/assets/placeholders/cover-generic.svg");
const banner=(game:Game)=>asset(game.local_banner_path||game.banner_url||game.banner_path)||asset("/assets/placeholders/banner-generic.svg");
const size=(bytes?:number|null)=>bytes?new Intl.NumberFormat("tr-TR",{style:"unit",unit:"megabyte",maximumFractionDigits:1}).format(bytes/1048576):"—";
const date=(value?:string|null)=>value?new Intl.DateTimeFormat("tr-TR",{dateStyle:"medium"}).format(new Date(value)):"—";
const dateTime=(value?:string|null)=>{
  if(!value)return "—";
  const normalized=/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(value)?value.replace(" ","T")+"Z":value;
  const parsed=new Date(normalized);
  return Number.isNaN(parsed.getTime())?value:new Intl.DateTimeFormat("tr-TR",{dateStyle:"short",timeStyle:"short"}).format(parsed);
};
const logoMarkup=(config:LoaderConfig,className="brand-logo")=>config.logo_url
  ? `<span class="${className} has-image"><img src="${escapeHtml(asset(config.logo_url))}" alt="" onerror="this.parentElement?.classList.remove('has-image');this.remove()"><b>A</b></span>`
  : `<span class="${className}"><b>A</b></span>`;


/** Sistem tarayıcısında açar; başarısız olursa kullanıcıya adresi gösterir. */
async function openLink(url?:string|null,missing="Bağlantı henüz tanımlanmadı."){
  if(!url){notify(missing,true);return}
  const absolute=/^https?:\/\//i.test(url)?url:WEB_BASE_URL+(url.startsWith("/")?url:"/"+url);
  try{await patchService.openExternal(absolute)}catch{notify("Bağlantı açılamadı: "+absolute,true)}
}


function userInitial(){
  return (state.user?.display_name||state.user?.email||"A")
    .trim()
    .charAt(0)
    .toLocaleUpperCase("tr-TR");
}

function avatarMarkup(className=""){
  const path=state.user?.avatar_path;
  if(path){
    return `<span class="${className}"><img src="${escapeHtml(asset(path))}" alt="Profil fotoğrafı"></span>`;
  }
  return `<span class="${className}">${escapeHtml(userInitial())}</span>`;
}

function syncProfileHeader(){
  const trigger=document.querySelector<HTMLButtonElement>("#profile-button");
  if(trigger){
    trigger.innerHTML=state.user?.avatar_path
      ? `<img src="${escapeHtml(asset(state.user.avatar_path))}" alt="Profil fotoğrafı">`
      : `<span>${escapeHtml(userInitial())}</span>`;
  }

  const name=document.querySelector<HTMLElement>("#header-profile-name");
  if(name)name.textContent=state.user?.display_name||"";

  const cardName=document.querySelector<HTMLElement>(".profile-card b");
  if(cardName)cardName.textContent=state.user?.display_name||"";

  const cardEmail=document.querySelector<HTMLElement>(".profile-card small");
  if(cardEmail)cardEmail.textContent=state.user?.email||"";
}

function openProfileSettings(){
  const modal=document.querySelector<HTMLElement>("#modal-root");
  if(!modal||!state.user)return;

  const avatar=state.user.avatar_path
    ? `<img src="${escapeHtml(asset(state.user.avatar_path))}" alt="Profil fotoğrafı">`
    : escapeHtml(userInitial());

  modal.innerHTML=`<div class="modal-backdrop" id="profile-settings-backdrop"><section class="profile-settings" role="dialog" aria-modal="true" aria-label="Profil ayarları"><button class="modal-close" id="profile-settings-close">×</button><div class="profile-settings-head"><div class="profile-settings-avatar" id="profile-settings-avatar">${avatar}</div><div><span class="overline">HESAP AYARLARI</span><h2>${escapeHtml(state.user.display_name)}</h2><p>${escapeHtml(state.user.email)} · ${escapeHtml(state.user.role)}</p></div></div><div class="profile-settings-body"><section class="profile-panel"><h3>Profil fotoğrafı</h3><p>JPG, PNG veya WebP. En fazla 5 MB.</p><div class="profile-avatar-actions"><input id="profile-avatar-input" type="file" accept="image/jpeg,image/png,image/webp"><button id="profile-avatar-select" type="button">Fotoğraf Seç</button>${state.user.avatar_path?'<button class="danger" id="profile-avatar-remove" type="button">Fotoğrafı Kaldır</button>':""}</div><div class="profile-avatar-note" id="profile-avatar-note">Fotoğraf seçildiğinde otomatik olarak yüklenir.</div></section><section class="profile-panel"><h3>Profil bilgileri</h3><p>Loader içinde görünen adını buradan değiştirebilirsin.</p><form class="profile-form" id="profile-info-form"><label>Görünen ad<input id="profile-display-name" minlength="2" maxlength="100" value="${escapeHtml(state.user.display_name)}" required></label><label>E-posta<input value="${escapeHtml(state.user.email)}" readonly></label><div class="profile-form-actions"><button type="submit" id="profile-info-save">Profili Kaydet</button><span class="profile-save-note" id="profile-info-note"></span></div></form></section><section class="profile-panel"><h3>Şifre değiştir</h3><p>Yeni şifre en az 12 karakter; büyük harf, küçük harf ve sayı içermeli.</p><form class="profile-form" id="profile-password-form"><label>Mevcut şifre<input id="profile-current-password" type="password" autocomplete="current-password" required></label><label>Yeni şifre<input id="profile-new-password" type="password" minlength="12" autocomplete="new-password" required></label><label>Yeni şifre tekrar<input id="profile-new-password-confirm" type="password" minlength="12" autocomplete="new-password" required></label><div class="profile-form-actions"><button type="submit" id="profile-password-save">Şifreyi Değiştir</button><span class="profile-save-note" id="profile-password-note"></span></div></form></section></div></section></div>`;

  const close=()=>{modal.innerHTML=""};
  document.querySelector("#profile-settings-close")?.addEventListener("click",close);
  document.querySelector("#profile-settings-backdrop")?.addEventListener("click",event=>{
    if(event.target===event.currentTarget)close();
  });

  const avatarInput=document.querySelector<HTMLInputElement>("#profile-avatar-input")!;
  document.querySelector("#profile-avatar-select")?.addEventListener("click",()=>avatarInput.click());

  avatarInput.addEventListener("change",async()=>{
    const file=avatarInput.files?.[0];
    if(!file)return;

    if(file.size>5*1024*1024){
      notify("Profil fotoğrafı en fazla 5 MB olabilir.",true);
      avatarInput.value="";
      return;
    }

    const note=document.querySelector<HTMLElement>("#profile-avatar-note");
    if(note)note.textContent="Fotoğraf yükleniyor…";

    try{
      state.user=await api.uploadAvatar(file);
      syncProfileHeader();
      if(note)note.textContent="Profil fotoğrafı güncellendi.";
      const preview=document.querySelector<HTMLElement>("#profile-settings-avatar");
      if(preview){
        preview.innerHTML=state.user.avatar_path
          ? `<img src="${escapeHtml(asset(state.user.avatar_path))}?v=${Date.now()}" alt="Profil fotoğrafı">`
          : escapeHtml(userInitial());
      }
      document.querySelector("#profile-avatar-remove")?.remove();
      const actions=document.querySelector<HTMLElement>(".profile-avatar-actions");
      if(actions&&state.user.avatar_path){
        const remove=document.createElement("button");
        remove.id="profile-avatar-remove";
        remove.type="button";
        remove.className="danger";
        remove.textContent="Fotoğrafı Kaldır";
        actions.append(remove);
        bindRemoveAvatar(remove);
      }
    }catch(error){
      if(note)note.textContent="";
      notify(error instanceof Error?error.message:"Profil fotoğrafı yüklenemedi.",true);
    }finally{
      avatarInput.value="";
    }
  });

  const bindRemoveAvatar=(button:HTMLElement)=>{
    button.addEventListener("click",async()=>{
      if(!confirm("Profil fotoğrafı kaldırılsın mı?"))return;
      const target=button as HTMLButtonElement;
      target.disabled=true;
      try{
        state.user=await api.removeAvatar();
        syncProfileHeader();
        const preview=document.querySelector<HTMLElement>("#profile-settings-avatar");
        if(preview)preview.textContent=userInitial();
        button.remove();
        const note=document.querySelector<HTMLElement>("#profile-avatar-note");
        if(note)note.textContent="Profil fotoğrafı kaldırıldı.";
      }catch(error){
        target.disabled=false;
        notify(error instanceof Error?error.message:"Profil fotoğrafı kaldırılamadı.",true);
      }
    });
  };

  const removeButton=document.querySelector<HTMLElement>("#profile-avatar-remove");
  if(removeButton)bindRemoveAvatar(removeButton);

  document.querySelector<HTMLFormElement>("#profile-info-form")!.addEventListener("submit",async event=>{
    event.preventDefault();
    const button=document.querySelector<HTMLButtonElement>("#profile-info-save")!;
    const note=document.querySelector<HTMLElement>("#profile-info-note")!;
    const displayName=document.querySelector<HTMLInputElement>("#profile-display-name")!.value.trim();
    button.disabled=true;
    note.textContent="Kaydediliyor…";
    try{
      state.user=await api.updateProfile(displayName);
      syncProfileHeader();
      note.textContent="Kaydedildi.";
      const title=document.querySelector<HTMLElement>(".profile-settings-head h2");
      if(title)title.textContent=state.user.display_name;
    }catch(error){
      note.textContent="";
      notify(error instanceof Error?error.message:"Profil güncellenemedi.",true);
    }finally{
      button.disabled=false;
    }
  });

  document.querySelector<HTMLFormElement>("#profile-password-form")!.addEventListener("submit",async event=>{
    event.preventDefault();
    const current=document.querySelector<HTMLInputElement>("#profile-current-password")!.value;
    const next=document.querySelector<HTMLInputElement>("#profile-new-password")!.value;
    const confirmPassword=document.querySelector<HTMLInputElement>("#profile-new-password-confirm")!.value;
    const button=document.querySelector<HTMLButtonElement>("#profile-password-save")!;
    const note=document.querySelector<HTMLElement>("#profile-password-note")!;

    if(next!==confirmPassword){
      notify("Yeni şifreler birbiriyle eşleşmiyor.",true);
      return;
    }

    button.disabled=true;
    note.textContent="Şifre değiştiriliyor…";

    try{
      const result=await api.changePassword(current,next);
      note.textContent=result.message||"Şifre değiştirildi.";
      document.querySelector<HTMLInputElement>("#profile-current-password")!.value="";
      document.querySelector<HTMLInputElement>("#profile-new-password")!.value="";
      document.querySelector<HTMLInputElement>("#profile-new-password-confirm")!.value="";
    }catch(error){
      note.textContent="";
      notify(error instanceof Error?error.message:"Şifre değiştirilemedi.",true);
    }finally{
      button.disabled=false;
    }
  });
}

function fallbackConfig():LoaderConfig{return cache.readConfig()||{app_name:"Animus Türkçe Yama",accent_color:"#b7f34a",library_title:"Oyun Kütüphanesi",announcements:[]}}

function loginView(error="",retry=false){
  const config=state.config||fallbackConfig();document.documentElement.style.setProperty("--accent",config.accent_color||"#b7f34a");
  const background="";
  app.innerHTML=`<main class="login"><section class="login-visual" style="${background?`background-image:linear-gradient(120deg,#08070ce8,#241238c9),url('${escapeHtml(background)}')`:""}">${logoMarkup(config,"loader-logo")}<span class="overline">GÜVENLİ TÜRKÇE YAMA PLATFORMU</span><h1>Oyunlarını Türkçe keşfet.</h1><p>Tek loader. Doğrulanmış indirme. Otomatik yedek, rollback ve güvenli kaldırma.</p></section><form id="login-form" class="login-form"><div class="mobile-logo">${escapeHtml(config.app_name)}</div><h2>Tekrar hoş geldin</h2><p>Kütüphanene erişmek için hesabınla devam et.</p>${error?`<div class="error"><b>Bağlantı kurulamadı</b><span>${escapeHtml(error)}</span>${retry?'<button type="button" id="retry-login">Tekrar Dene</button>':""}</div>`:""}<label>E-posta<input name="email" type="email" required autocomplete="username"></label><label>Şifre<div class="password-field"><input name="password" type="password" required autocomplete="current-password"><button id="toggle-password" type="button" aria-label="Şifreyi göster">Göster</button></div></label><label class="remember"><input name="remember" type="checkbox" checked> <span>Beni hatırla</span></label><button class="login-submit">Giriş Yap <span>→</span></button><div class="login-links"><button type="button" id="forgot-password">Şifremi Unuttum</button><button type="button" id="create-account">Hesap Oluştur</button></div><small>Şifreniz diske kaydedilmez. Oturum anahtarı Windows Credential Manager ile korunur.</small></form></main>`;
  const loginBackground=config.branding?.login_background||(config.login_background_url?{type:"image",image_url:config.login_background_url,overlay:60}:undefined);
  mountBackgroundMedia(document.querySelector<HTMLElement>(".login-visual")!,loginBackground,asset,60);
  document.querySelector<HTMLButtonElement>("#toggle-password")!.onclick=()=>{const input=document.querySelector<HTMLInputElement>('input[name="password"]')!;input.type=input.type==="password"?"text":"password";document.querySelector<HTMLButtonElement>("#toggle-password")!.textContent=input.type==="password"?"Göster":"Gizle"};
  document.querySelector("#retry-login")?.addEventListener("click",()=>start());
  document.querySelector("#create-account")?.addEventListener("click",()=>registerView());
  document.querySelector("#forgot-password")?.addEventListener("click",()=>void openLink(WEB_BASE_URL+"/forgot-password","Şifre sıfırlama adresi tanımlı değil."));
  document.querySelector<HTMLFormElement>("#login-form")!.onsubmit=async event=>{event.preventDefault();const form=new FormData(event.currentTarget as HTMLFormElement);const submit=document.querySelector<HTMLButtonElement>(".login-submit")!;submit.disabled=true;submit.textContent="Bağlanıyor…";try{state.user=await api.login(String(form.get("email")),String(form.get("password")),form.has("remember"));await boot()}catch(e){loginView(e instanceof Error?e.message:"Giriş başarısız",true)}};
}

function registerView(error="",retry=false){
  const config=state.config||fallbackConfig();document.documentElement.style.setProperty("--accent",config.accent_color||"#b7f34a");
  app.innerHTML=`<main class="login"><section class="login-visual">${logoMarkup(config,"loader-logo")}<span class="overline">GÜVENLİ TÜRKÇE YAMA PLATFORMU</span><h1>Animus'a katıl.</h1><p>Tek loader üzerinden hesabını oluştur, oyun kütüphanene ve sana açık Türkçe yamalara eriş.</p></section><form id="register-form" class="login-form register-form"><div class="mobile-logo">${escapeHtml(config.app_name)}</div><h2>Hesap oluştur</h2><p>Kayıt işlemi güvenli Animus API üzerinden tamamlanır.</p>${error?`<div class="error"><b>Kayıt tamamlanamadı</b><span>${escapeHtml(error)}</span>${retry?'<button type="button" id="retry-register">Tekrar Dene</button>':""}</div>`:""}<label>Görünen ad<input name="display_name" minlength="2" maxlength="100" required autocomplete="name"></label><label>E-posta<input name="email" type="email" required autocomplete="email"></label><label>Şifre<input name="password" type="password" minlength="12" required autocomplete="new-password"></label><label>Şifre tekrar<input name="password_confirm" type="password" minlength="12" required autocomplete="new-password"></label><button class="login-submit">Kayıt Ol <span>→</span></button><div class="login-links auth-switch"><span>Zaten hesabın var mı?</span><button type="button" id="back-to-login">Giriş Yap</button></div><small>Şifreniz diske kaydedilmez. Oturum anahtarı Windows Credential Manager ile korunur.</small></form></main>`;
  const background=config.branding?.login_background||(config.login_background_url?{type:"image",image_url:config.login_background_url,overlay:60}:undefined);mountBackgroundMedia(document.querySelector<HTMLElement>(".login-visual")!,background,asset,60);
  document.querySelector("#back-to-login")!.addEventListener("click",()=>loginView());document.querySelector("#retry-register")?.addEventListener("click",()=>registerView());
  document.querySelector<HTMLFormElement>("#register-form")!.onsubmit=async event=>{event.preventDefault();const form=new FormData(event.currentTarget as HTMLFormElement);const input={displayName:String(form.get("display_name")||""),email:String(form.get("email")||""),password:String(form.get("password")||""),passwordConfirm:String(form.get("password_confirm")||"")};const validation=validateRegistration(input);if(validation){registerView(validation);return}const submit=document.querySelector<HTMLButtonElement>(".login-submit")!;submit.disabled=true;submit.textContent="Hesap oluşturuluyor…";try{state.user=await api.register(input.displayName,input.email,input.password);await boot()}catch(e){registerView(e instanceof Error?e.message:"Kayıt tamamlanamadı",true)}};
}

function shell(){
  const config=state.config||fallbackConfig();document.documentElement.style.setProperty("--accent",config.accent_color||"#b7f34a");
  const subscription=state.user?.subscription;
  app.innerHTML=`<div class="shell${state.update?" has-update":""}"><header><button class="brand nav-button" data-view="home">${logoMarkup(config)}${escapeHtml(config.app_name)}</button><nav><button class="nav-button active" data-view="home">Ana Sayfa</button><button class="nav-button" data-view="library">Kütüphane</button><button id="backup-nav">Yedekler</button><button class="nav-button" data-view="emulator">PS Oyun Emülatör</button><button class="nav-button" data-view="announcements">Duyurular</button><button class="nav-button" data-view="chat">Canlı Chat</button></nav><div class="user"><div class="user-profile-copy"><b id="header-profile-name">${escapeHtml(state.user?.display_name)}</b><small>${state.user?.premium?escapeHtml(subscription?.plan_name||"Premium"):"Ücretsiz"}${subscription?.ends_at?" · "+date(subscription.ends_at):""}</small></div><button id="profile-button" class="profile-trigger" type="button" title="Profil ayarları">${state.user?.avatar_path?`<img src="${escapeHtml(asset(state.user.avatar_path))}" alt="Profil fotoğrafı">`:`<span>${escapeHtml(userInitial())}</span>`}</button><button id="logout">Çıkış</button></div></header>${state.update?`<div class="update-banner"><b>Yeni loader sürümü hazır: ${escapeHtml(state.update.version)}</b><span>Kurulu sürüm ${escapeHtml(state.update.currentVersion)}</span><button id="update-banner-action">Şimdi Güncelle</button></div>`:""}<main id="main-content"></main><aside class="rightbar"><section class="profile-card"><span>${state.user?.premium?"PREMIUM":"FREE"}</span><b>${escapeHtml(state.user?.display_name)}</b><small>${escapeHtml(state.user?.email)}</small></section><h3>Duyurular</h3><div id="announcements"></div><section class="translation-summary"><h3>Çeviri İlerlemeleri</h3><div id="translation-summary"></div></section><div class="support-card"><span>YARDIMA MI İHTİYACIN VAR?</span><b>Destek merkezine ulaş</b><button id="support-link">Destek →</button><div class="social-links" id="social-links" hidden></div></div><div class="loader-version">Loader v${escapeHtml(state.loaderVersion)}</div></aside></div><div id="modal-root"></div><div id="dev-panel"></div>`;
  if(canManage(state.user)){const button=document.createElement("button");button.className="nav-button";button.dataset.view="admin";button.textContent="Yönetim";document.querySelector(".shell nav")?.append(button)}
  mountBackgroundMedia(document.querySelector<HTMLElement>(".shell")!,config.branding?.library_background,asset,55);
  document.querySelector("#profile-button")?.addEventListener("click",openProfileSettings);
  document.querySelector("#logout")!.addEventListener("click",async()=>{stopChatPolling();try{await api.logout()}catch{}state.user=null;state.selected=null;installations.clear();loginView()});
  document.querySelector("#backup-nav")!.addEventListener("click",showBackups);
  document.querySelector("#support-link")!.addEventListener("click",()=>void openLink(config.support_url,"Destek bağlantısı henüz tanımlanmadı."));
  const social=document.querySelector<HTMLElement>("#social-links");
  if(social){
    ([["Discord",config.discord_url],["YouTube",config.youtube_url],["Instagram",config.instagram_url],["X",config.x_url]] as [string,string|undefined][])
      .filter(([,url])=>Boolean(url))
      .forEach(([label,url])=>{const button=document.createElement("button");button.textContent=label;button.onclick=()=>void openLink(url);social.append(button)});
    social.hidden=social.childElementCount===0;
  }
  document.querySelector("#update-banner-action")?.addEventListener("click",()=>void applyLoaderUpdate());
  document.querySelectorAll<HTMLButtonElement>(".nav-button").forEach(button=>button.onclick=()=>showView(button.dataset.view||"home"));
  document.querySelector("#announcements")!.innerHTML=(config.announcements||[]).map(a=>`<article><b>${escapeHtml(a.title)}</b><p>${escapeHtml(a.body)}</p></article>`).join("")||'<p class="muted">Yeni duyuru yok.</p>';
  document.querySelector("#translation-summary")!.innerHTML=state.games.slice().sort((a,b)=>b.translation_percent-a.translation_percent).slice(0,4).map(game=>`<article><span>${escapeHtml(game.name)}</span><div class="progress"><i style="width:${game.translation_percent}%"></i></div><b>%${game.translation_percent}</b></article>`).join("");
  window.removeEventListener("keydown",developerShortcut);
  window.addEventListener("keydown",developerShortcut);
  showView("home");
}

function developerShortcut(event:KeyboardEvent){if(event.ctrlKey&&event.shiftKey&&event.key.toLowerCase()==="d"){state.developer=!state.developer;renderDeveloper()}}
function setActiveNav(view:string){document.querySelectorAll<HTMLButtonElement>(".nav-button").forEach(button=>button.classList.toggle("active",button.dataset.view===view))}
function stopChatPolling(){
  if(chatPollTimer!==undefined){window.clearInterval(chatPollTimer);chatPollTimer=undefined}
  chatRefreshRunning=false;
}

function showView(view:string){
  if(view!=="chat")stopChatPolling();

  if(view==="admin"&&canManage(state.user)){currentView="admin";setActiveNav(currentView);void renderAdminPanel(document.querySelector<HTMLElement>("#main-content")!,async()=>{state.games=await api.games();cache.saveGames(state.games);await loadConfig()},notify);return}
  if(view==="emulator"){currentView="emulator";setActiveNav(currentView);renderEmulator();return}
  if(view==="announcements"){currentView="announcements";setActiveNav(currentView);void renderAnnouncements();return}
  if(view==="chat"){currentView="chat";setActiveNav(currentView);renderChat();return}

  currentView=view==="library"?"library":"home";
  setActiveNav(currentView);
  currentView==="library"?renderLibrary():renderHome();
}

async function renderAnnouncements(){
  const host=document.querySelector<HTMLElement>("#main-content")!;
  host.innerHTML=`<section class="community-page"><div class="community-head"><div><span class="overline">ANIMUS TOPLULUK</span><h1>Duyurular</h1><p>Animus ekibinden yayınlanan güncel bilgilendirmeler.</p></div><button id="announcement-refresh">Yenile</button></div><div id="announcement-list" class="announcement-list"><div class="community-empty">Duyurular yükleniyor…</div></div></section>`;

  document.querySelector("#announcement-refresh")?.addEventListener("click",()=>void renderAnnouncements());

  try{
    const items=await api.announcements();
    if(currentView!=="announcements")return;
    const list=document.querySelector<HTMLElement>("#announcement-list");
    if(!list)return;
    list.innerHTML=items.length?items.map(item=>{
      const audienceLabels:Record<string,string>={all:"Herkes",free:"Ücretsiz",premium:"Premium",tester:"Tester",admin:"Yönetim"};
      return `<article class="announcement-card"><header><div><div class="announcement-meta"><span class="audience-badge">${escapeHtml(audienceLabels[item.audience]||item.audience)}</span><span>${escapeHtml(dateTime(item.created_at))}</span></div><h2>${escapeHtml(item.title)}</h2></div></header><p>${escapeHtml(item.body)}</p></article>`;
    }).join(""):'<div class="community-empty">Şu anda gösterilecek duyuru yok.</div>';
  }catch(error){
    const list=document.querySelector<HTMLElement>("#announcement-list");
    if(list)list.innerHTML=`<div class="community-empty">${escapeHtml(error instanceof Error?error.message:"Duyurular yüklenemedi.")}</div>`;
  }
}

function chatCanModerate(){
  return state.user?.role==="admin"||state.user?.role==="super_admin";
}

async function refreshChat(forceBottom=false){
  if(currentView!=="chat"||chatRefreshRunning)return;
  const messagesHost=document.querySelector<HTMLElement>("#chat-messages");
  if(!messagesHost)return;

  chatRefreshRunning=true;
  const status=document.querySelector<HTMLElement>("#chat-status");
  const oldTop=messagesHost.scrollTop;
  const wasNearBottom=messagesHost.scrollHeight-messagesHost.scrollTop-messagesHost.clientHeight<90;

  try{
    const items=await api.chatMessages();
    if(currentView!=="chat")return;

    const myId=Number(state.user?.id||0);
    const canModerate=chatCanModerate();

    messagesHost.innerHTML=items.length?items.map(item=>{
      const mine=item.user_id===myId;
      const staff=item.role==="admin"||item.role==="super_admin";
      const initial=(item.display_name||"?").trim().charAt(0).toLocaleUpperCase("tr-TR");
      return `<article class="chat-message ${mine?"mine":""}" data-message-id="${item.id}"><div class="chat-avatar">${escapeHtml(initial)}</div><div class="chat-bubble"><div class="chat-name"><b>${escapeHtml(item.display_name)}</b><span class="chat-role ${staff?"staff":""}">${escapeHtml(item.role)}</span></div><p>${escapeHtml(item.body)}</p><small class="chat-time">${escapeHtml(dateTime(item.created_at))}</small></div>${canModerate?`<button class="chat-delete" data-id="${item.id}" title="Mesajı sil">Sil</button>`:""}</article>`;
    }).join(""):'<div class="community-empty">Henüz mesaj yok. İlk mesajı sen gönder.</div>';

    if(forceBottom||wasNearBottom)messagesHost.scrollTop=messagesHost.scrollHeight;
    else messagesHost.scrollTop=oldTop;

    if(status)status.textContent="Bağlı · "+items.length+" mesaj";
  }catch{
    if(status)status.textContent="Bağlantı yenileniyor…";
  }finally{
    chatRefreshRunning=false;
  }
}

function renderChat(){
  stopChatPolling();
  const host=document.querySelector<HTMLElement>("#main-content")!;
  host.innerHTML=`<section class="chat-page"><div class="community-head"><div><span class="overline">ANIMUS TOPLULUK</span><h1>Canlı Chat</h1><p>Animus hesabınla topluluk sohbetine katıl.</p></div></div><div class="chat-shell"><div class="chat-toolbar"><div class="chat-live"><i></i><b>CANLI</b></div><span class="chat-status" id="chat-status">Bağlanıyor…</span></div><div class="chat-messages" id="chat-messages"><div class="community-empty">Mesajlar yükleniyor…</div></div><div class="chat-compose"><form id="chat-form"><textarea id="chat-input" maxlength="500" placeholder="Mesajını yaz…" required></textarea><button id="chat-send" type="submit">Gönder</button></form><div class="chat-compose-foot"><span>Topluluk kurallarına uygun iletişim kur.</span><span id="chat-count">0 / 500</span></div></div></div></section>`;

  const input=document.querySelector<HTMLTextAreaElement>("#chat-input")!;
  const form=document.querySelector<HTMLFormElement>("#chat-form")!;
  const count=document.querySelector<HTMLElement>("#chat-count")!;
  const messagesHost=document.querySelector<HTMLElement>("#chat-messages")!;

  input.addEventListener("input",()=>{count.textContent=`${input.value.length} / 500`});
  input.addEventListener("keydown",event=>{
    if(event.key==="Enter"&&!event.shiftKey){event.preventDefault();form.requestSubmit()}
  });

  messagesHost.addEventListener("click",event=>{
    const button=(event.target as HTMLElement).closest<HTMLButtonElement>(".chat-delete");
    if(!button)return;
    const id=Number(button.dataset.id);
    if(!id||!confirm("Bu chat mesajı silinsin mi?"))return;
    button.disabled=true;
    void api.deleteChat(id).then(()=>refreshChat()).catch(error=>{notify(error instanceof Error?error.message:"Mesaj silinemedi.",true);button.disabled=false});
  });

  form.addEventListener("submit",async event=>{
    event.preventDefault();
    const body=input.value.trim();
    if(!body)return;
    const send=document.querySelector<HTMLButtonElement>("#chat-send")!;
    send.disabled=true;
    try{
      await api.sendChat(body);
      input.value="";
      count.textContent="0 / 500";
      await refreshChat(true);
      input.focus();
    }catch(error){
      notify(error instanceof Error?error.message:"Mesaj gönderilemedi.",true);
    }finally{
      send.disabled=false;
    }
  });

  void refreshChat(true);
  chatPollTimer=window.setInterval(()=>void refreshChat(),2500);
  input.focus();
}

function renderHome(){
  const config=state.config||fallbackConfig();const featured=(config.banners||[])[0];const background=asset(featured?.image_path||config.banner_url)||asset("/assets/placeholders/banner-generic.svg");
  const patched=state.games.filter(game=>game.patch_version);const recent=patched.slice().sort((a,b)=>String(b.published_at||"").localeCompare(String(a.published_at||""))).slice(0,5);const popular=state.games.slice(0,5);
  document.querySelector("#main-content")!.innerHTML=`<section class="hero launcher-hero" style="background-image:linear-gradient(90deg,#09070df2,#15101bbb),url('${escapeHtml(background)}')"><div><span class="overline">ANIMUS KÜTÜPHANESİ</span><h1>${escapeHtml(featured?.title||config.library_title)}</h1><p>Yeni oyunlar ve yayınlanan Türkçe yamalar loader yeniden derlenmeden burada görünür.</p><div class="hero-actions"><button id="browse">Kütüphaneyi Aç</button><button class="ghost" id="refresh">Kataloğu Yenile</button></div></div></section><section class="home-section"><div class="section-title"><h2>Yeni Eklenen Yamalar</h2><button data-library>Hepsini Gör</button></div><div class="horizontal-games">${gameCards(recent)}</div></section><section class="home-section"><div class="section-title"><h2>Son Güncellenenler</h2></div><div class="horizontal-games">${gameCards(recent)}</div></section><section class="home-section"><div class="section-title"><h2>Popüler Oyunlar</h2></div><div class="horizontal-games">${gameCards(popular)}</div></section>`;
  document.querySelector("#browse")!.addEventListener("click",()=>showView("library"));document.querySelectorAll("[data-library]").forEach(b=>b.addEventListener("click",()=>showView("library")));
  document.querySelector("#refresh")!.addEventListener("click",()=>loadGames(true));bindGameCards();
}


type PsPlatform="ps1"|"ps2";

type ManagedPsGame={
  platform:PsPlatform;
  platformLabel:string;
  title:string;
  emulator:string;
  match:(game:Game)=>boolean;
};

const psGames:ManagedPsGame[]=[
  {
    platform:"ps1",
    platformLabel:"PlayStation 1",
    title:"Silent Hill",
    emulator:"DuckStation",
    match:(game:Game)=>/^silent hill(?: 1)?$/i.test(game.name.trim())
  },
  {
    platform:"ps2",
    platformLabel:"PlayStation 2",
    title:"Resident Evil Code: Veronica",
    emulator:"PCSX2",
    match:(game:Game)=>/resident evil.*code[: ]?\s*veronica/i.test(game.name)
  }
];

function psConfig(game:Game){
  return psGames.find(item=>item.match(game));
}

function isManagedPsGame(game:Game){
  return Boolean(psConfig(game));
}

function psCatalogGame(item:ManagedPsGame){
  return state.games.find(item.match);
}

/** PS oyunlarında kök klasörü kullanıcı seçmez; Rust tarafı LocalAppData altında oluşturur. */
async function managedPsRoot(game:Game){
  const installedRoot=installations.root(game.id);
  if(installedRoot)return installedRoot;

  const root=await invoke<string>("prepare_ps_game_root",{gameId:game.id});
  localStorage.setItem("root_"+game.id,root);
  return root;
}

/** Kurulmuş PS paketinin içindeki ISO/CUE/CHD dosyasını Rust otomatik bulur ve emülatörde açar. */
async function launchInstalledPsGame(game:Game,platform:PsPlatform){
  if(!installations.version(game.id)){
    notify("Önce OYUNU HAZIRLA ile oyun paketini kur.",true);
    return;
  }

  try{
    const root=await managedPsRoot(game);
    const image=await invoke<string>("launch_installed_ps_game",{platform,gameRoot:root});
    notify(`${image} başlatılıyor…`);
  }catch(error){
    notify("Oyun başlatılamadı: "+(error instanceof Error?error.message:String(error)),true);
  }
}


/** Kullanıcının kendi PS1/PS2 BIOS dump'ını Animus'un yönetilen emülatör klasörüne ekler. */
async function installPsBios(platform:PsPlatform){
  try{
    const command=platform==="ps1"?"install_ps1_bios":"install_ps2_bios";
    const path=await invoke<string|null>(command);
    if(!path)return;
    notify(`${platform==="ps1"?"PS1":"PS2"} BIOS eklendi. Artık oyunu başlatabilirsin.`);
  }catch(error){
    notify("BIOS eklenemedi: "+(error instanceof Error?error.message:String(error)),true);
  }
}

function renderEmulator(){
  const cards=psGames.map(item=>{
    const game=psCatalogGame(item);

    if(!game){
      return `<article class="game-card emulator-game-card">
        <div class="cover" style="background-image:url('${escapeHtml(asset("/assets/placeholders/cover-generic.svg"))}')">
          <span class="access free">${escapeHtml(item.platformLabel)}</span>
          <span class="install-badge">KATALOĞA EKLENMEDİ</span>
        </div>
        <h3>${escapeHtml(item.title)}</h3>
        <div class="meta"><span>${escapeHtml(item.emulator)}</span><b>${item.platform.toUpperCase()}</b></div>
      </article>`;
    }

    const installed=installations.version(game.id);
    const update=installations.hasUpdate(game.id,game.patch_version);
    const noPatch=!game.patch_version_id;
    const status=update?"GÜNCELLEME VAR":installed?"OYNAMAYA HAZIR":game.patch_version?"OYUNU HAZIRLA":"PAKET BEKLENİYOR";
    const biosButton=`<button class="ghost ps-bios-add" data-id="${game.id}" data-platform="${item.platform}">BIOS EKLE</button>`;

    return `<article class="game-card emulator-game-card">
      <div class="cover" style="background-image:url('${escapeHtml(cover(game))}')">
        <span class="access free">${escapeHtml(item.platformLabel)}</span>
        <span class="install-badge ${installed?"installed":""}">${status}</span>
      </div>
      <h3>${escapeHtml(game.name)}</h3>
      <div class="meta"><span>${escapeHtml(item.emulator)} ile çalışır</span><b>${item.platform.toUpperCase()}</b></div>
      <div class="detail-actions" style="margin-top:12px">
        ${
          installed
            ? `<button class="ps-play-installed" data-id="${game.id}" data-platform="${item.platform}">OYNA</button>${update?`<button class="ghost ps-prepare" data-id="${game.id}">OYUNU GÜNCELLE</button>`:""}${biosButton}`
            : `<button class="ps-prepare" data-id="${game.id}" ${noPatch?"disabled":""}>OYUNU HAZIRLA</button>${biosButton}`
        }
      </div>
    </article>`;
  }).join("");

  document.querySelector("#main-content")!.innerHTML=`<section class="catalog full-library">
    <div class="catalog-head">
      <div>
        <span class="overline">ANIMUS EMU</span>
        <h1>PS Oyun Emülatör</h1>
        <p class="muted">PlayStation oyun paketi MediaFire üzerinden Animus tarafından hazırlanır. ISO, CUE veya CHD dosyasını ayrıca seçmen gerekmez.</p>
      </div>
    </div>
    <div class="game-grid">${cards}</div>
  </section>`;

  document.querySelectorAll<HTMLButtonElement>(".ps-prepare").forEach(button=>button.onclick=()=>{
    const game=state.games.find(item=>item.id===Number(button.dataset.id));
    if(game)void install(game);
  });

  document.querySelectorAll<HTMLButtonElement>(".ps-play-installed").forEach(button=>button.onclick=()=>{
    const game=state.games.find(item=>item.id===Number(button.dataset.id));
    if(game)void launchInstalledPsGame(game,button.dataset.platform as PsPlatform);
  });

  document.querySelectorAll<HTMLButtonElement>(".ps-bios-add").forEach(button=>button.onclick=()=>{
    void installPsBios(button.dataset.platform as PsPlatform);
  });
}

function renderLibrary(){
  document.querySelector("#main-content")!.innerHTML=`<section class="catalog full-library"><div class="catalog-head"><div><span class="overline">DİNAMİK API KATALOĞU</span><h1>Kütüphane</h1><div class="filters" id="filters"><button data-filter="all" class="active">Tümü</button><button data-filter="installed">Kurulu</button><button data-filter="update">Güncelleme Var</button><button data-filter="free">Ücretsiz</button><button data-filter="premium">Premium</button></div></div><input id="search" placeholder="Oyun adı ara…"></div><div id="game-grid" class="game-grid"></div></section>`;
  document.querySelector<HTMLInputElement>("#search")!.oninput=event=>{state.query=(event.target as HTMLInputElement).value;renderGames()};
  document.querySelectorAll<HTMLButtonElement>("#filters button").forEach(button=>button.onclick=()=>{document.querySelectorAll("#filters button").forEach(item=>item.classList.remove("active"));button.classList.add("active");state.filter=button.dataset.filter||"all";renderGames()});renderGames();
}

function filteredGames(){return filterCatalog(state.games,state.query,state.filter,id=>installations.version(id))}
/** Kurulum durumu değişince açık olan görünümü yeniden çizer. */
function refreshCurrentView(){
  if(currentView==="library")renderLibrary();
  else if(currentView==="emulator")renderEmulator();
  else if(currentView==="announcements")void renderAnnouncements();
  else if(currentView==="chat")renderChat();
  else renderHome();
}
function gameCards(games:Game[]){
  if(!games.length)return '<div class="empty-catalog">Bu bölümde henüz yayınlanmış yama bulunmuyor.</div>';

  return games.map(game=>{
    const installed=installations.version(game.id);
    const update=installations.hasUpdate(game.id,game.patch_version);
    const orphan=installations.isOrphaned(game.id);
    const ps=psConfig(game);

    if(orphan&&!ps){
      return `<article class="game-card" data-id="${game.id}"><div class="cover" style="background-image:url('${escapeHtml(cover(game))}')"><span class="access ${game.access_type}">${game.access_type==="premium"?"PREMIUM":"FREE"}</span><span class="install-badge orphan">Oyun klasörü bulunamadı</span><button>Detay →</button></div><h3>${escapeHtml(game.name)}</h3><div class="meta"><span>Kurulum kaydı var, klasör kayıp</span><b>%${game.translation_percent}</b></div><div class="progress"><i style="width:${game.translation_percent}%"></i></div></article>`;
    }

    const badge=ps
      ? (update?"Güncelleme Var":installed?"Oynamaya Hazır":game.patch_version?"Oyun Hazırla":"Paket Bekleniyor")
      : (update?"Güncelleme Var":installed?"Kurulu":game.patch_version?"Yama Hazır":"Yama Bekleniyor");

    const description=ps
      ? (game.patch_version?`Animus Emu paket ${game.patch_version}`:"Oyun paketi henüz yüklenmedi.")
      : (game.patch_version?`Yama ${game.patch_version}`:"Türkçe yama henüz yüklenmedi.");

    return `<article class="game-card" data-id="${game.id}"><div class="cover" style="background-image:url('${escapeHtml(cover(game))}')"><span class="access ${game.access_type}">${ps?escapeHtml(ps.platformLabel):(game.access_type==="premium"?"PREMIUM":"FREE")}</span><span class="install-badge ${installed?"installed":""}">${badge}</span><button>Detay →</button></div><h3>${escapeHtml(game.name)}</h3><div class="meta"><span>${escapeHtml(description)}</span><b>%${game.translation_percent}</b></div><div class="progress"><i style="width:${game.translation_percent}%"></i></div></article>`;
  }).join("");
}
function renderGames(){const grid=document.querySelector("#game-grid");if(!grid)return;grid.innerHTML=gameCards(filteredGames());bindGameCards()}
function bindGameCards(){document.querySelectorAll<HTMLElement>(".game-card").forEach(card=>card.onclick=()=>showGame(state.games.find(game=>game.id===Number(card.dataset.id))!))}

async function loadGames(force=false){try{state.games=await api.games();cache.saveGames(state.games);await log("info","catalog",`${state.games.length} oyun API üzerinden yüklendi`);if(document.querySelector(".shell"))shell();if(force)notify("Katalog güncellendi")}catch{state.games=cache.readGames();await log("warning","catalog",`API kullanılamadı; ${state.games.length} önbellek kaydı gösteriliyor`);if(document.querySelector(".shell"))shell();notify("Sunucuya ulaşılamadı; önbellekteki katalog gösteriliyor.",true)}}

async function showGame(summary:Game){
  let game=summary;
  try{game=await api.game(summary.id)}
  catch{notify("Oyun ayrıntıları güncellenemedi; önbellekteki bilgiler gösteriliyor.",true)}

  state.selected=game;

  const record=installations.get(game.id);
  const installed=installations.version(game.id);
  const update=installations.hasUpdate(game.id,game.patch_version);
  const ps=psConfig(game);
  const stores=ps?`${ps.platformLabel} · Animus Emu`:(game.supported_stores||[]).join(" · ")||"Manuel";
  const noPatch=!game.patch_version_id;
  const rootPath=installations.root(game.id)||localStorage.getItem("root_"+game.id)||"";
  const blocked=!meetsMinimum(state.loaderVersion,game.minimum_loader_version);

  const pathSection=ps
    ? `<div class="patch-unavailable" style="border-color:transparent;background:rgba(183,243,74,.06)">Oyun imajı MediaFire paketinden Animus tarafından otomatik hazırlanır. Oyun klasörü veya ISO/CUE/CHD seçmen gerekmez. ${ps.platformLabel} BIOS dosyanı bir kez BIOS EKLE ile tanımlaman gerekir.</div>`
    : `<label class="path-row">Oyun dizini<input id="game-root" readonly value="${escapeHtml(rootPath||"Otomatik bulunacak / manuel seçilebilir")}"><button id="select-root">OYUN KLASÖRÜNÜ SEÇ</button></label>`;

  const actions=ps
    ? (
        installed
          ? `<div class="detail-actions"><button id="ps-play-action">OYNA</button><button class="ghost" id="ps-bios-action">BIOS EKLE</button>${update?`<button class="ghost" id="install-action" ${noPatch||blocked?"disabled":""}>OYUNU GÜNCELLE</button>`:""}<button class="ghost" id="verify-action">DOSYALARI DOĞRULA</button><button class="danger" id="uninstall-action" ${record?"":"disabled"}>OYUNU KALDIR</button></div>`
          : `<div class="detail-actions"><button id="install-action" ${noPatch||blocked?"disabled":""}>OYUNU HAZIRLA</button><button class="ghost" id="ps-bios-action">BIOS EKLE</button></div>`
      )
    : `<div class="detail-actions"><button id="install-action" ${noPatch||blocked?"disabled":""}>${update?"YAMAYI GÜNCELLE":"YAMAYI KUR"}</button><button class="ghost" id="verify-action" ${installed?"":"disabled"}>DOSYALARI DOĞRULA</button><button class="danger" id="uninstall-action" ${record?"":"disabled"}>YAMAYI KALDIR</button></div>`;

  document.querySelector("#modal-root")!.innerHTML=`<div class="modal-backdrop"><section class="game-modal"><button class="modal-close">×</button><div class="detail-banner" style="background-image:linear-gradient(90deg,#0b0910f2,#0b091077),url('${escapeHtml(banner(game))}')"><img src="${escapeHtml(cover(game))}" alt=""><div><span class="overline">${escapeHtml(ps?ps.platformLabel:(game.categories.join(" · ")||"OYUN"))}</span><h2>${escapeHtml(game.name)}</h2><p>${escapeHtml(game.description||game.short_description||"Oyun bilgileri yönetim panelinden güncellenebilir.")}</p><span class="detail-access">${game.access_type==="premium"?"PREMIUM":"FREE"} · %${game.translation_percent} ÇEVİRİ</span></div></div><div class="detail-grid"><article><small>${ps?"Paket sürümü":"Yama sürümü"}</small><b>${escapeHtml(game.patch_version||"Henüz yüklenmedi")}</b></article><article><small>Desteklenen oyun</small><b>${escapeHtml(game.game_version||"Belirtilmedi")}</b></article><article><small>${ps?"Oyun paketi":"Patch"} boyutu</small><b>${size(game.size_bytes)}</b></article><article><small>Son güncelleme</small><b>${date(game.published_at)}</b></article><article><small>${ps?"Platform":"Mağaza"}</small><b>${escapeHtml(stores)}</b></article><article><small>Durum</small><b>${installed?(ps?"Oynamaya hazır":"Kurulu "+escapeHtml(installed)):"Kurulu değil"}</b></article></div><section class="changelog"><h3>Değişiklik Notları</h3><p>${escapeHtml(game.changelog||(noPatch?(ps?"Oyun paketi henüz yüklenmedi.":"Türkçe yama henüz yüklenmedi."):"Değişiklik notu yayınlanmadı."))}</p></section>${pathSection}${actions}${noPatch?`<div class="patch-unavailable">${ps?"Oyun paketi henüz yüklenmedi.":"Türkçe yama henüz yüklenmedi."}</div>`:""}${blocked?`<div class="patch-unavailable">Bu paket en az ${escapeHtml(game.minimum_loader_version)} sürümünde loader gerektiriyor. Kurulu sürüm: ${escapeHtml(state.loaderVersion)}.</div>`:""}${record&&!record.root_exists&&!ps?'<div class="patch-unavailable">Kurulum kaydı var ama oyun klasörü bulunamıyor. Klasörü geri getirin veya kaydı temizlemek için yamayı kaldırmayı deneyin.</div>':""}${record&&!record.backup_exists?`<div class="patch-unavailable">${ps?"Bu oyun paketinin":"Bu kurulumun"} yedeği bulunamıyor; kaldırma işlemi eksik kalabilir.</div>`:""}</section></div>`;

  document.querySelector(".modal-close")!.addEventListener("click",closeModal);

  if(!ps){
    document.querySelector("#select-root")?.addEventListener("click",async()=>{
      try{
        const required=game.executable?[game.executable]:[];
        const root=await patchService.chooseGameRoot(required);
        if(root){
          localStorage.setItem("root_"+game.id,root);
          const input=document.querySelector("#game-root") as HTMLInputElement|null;
          if(input)input.value=root;
        }
      }catch(error){notify(patchMessage(error),true)}
    });
  }

  document.querySelector("#install-action")?.addEventListener("click",()=>install(game));
  document.querySelector("#verify-action")?.addEventListener("click",()=>operation(async root=>patchService.verify(game.id,root),"Dosyalar doğrulandı"));
  document.querySelector("#uninstall-action")?.addEventListener("click",()=>uninstall(game));

  if(ps){
    document.querySelector("#ps-play-action")?.addEventListener("click",()=>void launchInstalledPsGame(game,ps.platform));
    document.querySelector("#ps-bios-action")?.addEventListener("click",()=>void installPsBios(ps.platform));
  }
}

async function ensureRoot(game:Game){
  if(isManagedPsGame(game)){
    return managedPsRoot(game);
  }

  // Normal PC oyunlarında mevcut davranış aynen devam eder.
  let root=installations.root(game.id)||localStorage.getItem("root_"+game.id);
  const required=game.executable?[game.executable]:[];
  if(!root&&game.steam_app_id){
    root=await patchService.detectGame(game.steam_app_id,required);
    if(root)localStorage.setItem("root_"+game.id,root);
  }
  if(!root)throw new Error("Oyun dizini bulunamadı. Lütfen oyun klasörünü manuel seçin.");
  return root;
}
async function operation(task:(root:string)=>Promise<unknown>,success:string,done?:()=>void){
  if(!state.selected)return;
  try{
    progress({stage:"prepare",percent:5,message:"Hazırlanıyor"});
    const root=await ensureRoot(state.selected);
    await task(root);
    done?.();
    await installations.refresh();
    progress({stage:"complete",percent:100,message:success});
    setTimeout(()=>{closeModal();refreshCurrentView()},900);
  }catch(error){progress({stage:"error",percent:0,message:patchMessage(error)},true)}
}

/** Kaldırma: dosyalar elle değiştirilmişse kullanıcıya zorla seçeneği sunulur. */
async function uninstall(game:Game){
  const ps=psConfig(game);
  try{
    progress({stage:"prepare",percent:5,message:ps?"Oyun paketi kaldırılıyor":"Yama kaldırılıyor"});
    const root=await ensureRoot(game);
    try{
      await patchService.uninstall(game.id,root,false);
    }catch(error){
      const message=error instanceof Error?error.message:String(error);
      if(!/değişmiş|yedeği bulunamadı/i.test(message))throw error;
      if(!confirm(message+"\n\nYedekten zorla geri yükleme denensin mi? Kendi değişiklikleriniz kaybolabilir."))
        {progress({stage:"error",percent:0,message:"Kaldırma iptal edildi."},true);return}
      await patchService.uninstall(game.id,root,true);
    }
    await installations.refresh();
    await log("info","uninstall",`Oyun #${game.id} ${ps?"paketi":"yaması"} kaldırıldı`);
    progress({stage:"complete",percent:100,message:ps?"Oyun paketi kaldırıldı":"Yama kaldırıldı"});
    setTimeout(()=>{closeModal();refreshCurrentView()},900);
  }catch(error){progress({stage:"error",percent:0,message:patchMessage(error)},true)}
}
async function install(game:Game){
  const ps=psConfig(game);
  try{
    if(!navigator.onLine)throw new Error("Kurulum için internet bağlantısı gerekli.");
    if(!game.patch_version_id)throw new Error(ps?"Oyun paketi henüz yüklenmedi.":"Türkçe yama henüz yüklenmedi.");

    const root=await ensureRoot(game);
    const patch=await api.patch(game.id);
    const id=Number(patch.id);
    const manifest=await api.manifest(id) as {patch?:{minimum_loader_version?:string}};

    const minimum=manifest.patch?.minimum_loader_version;
    if(!meetsMinimum(state.loaderVersion,minimum))
      throw new Error(`Bu paket en az ${minimum} sürümünde loader gerektiriyor. Kurulu sürüm: ${state.loaderVersion}.`);

    const dry=await patchService.dryRun(manifest,root);
    const existing=installations.version(game.id);
    const notice=existing?`Kurulu ${existing} sürümü önce kaldırılacak ve yeni sürüm hazırlanacak.\n\n`:"";

    const question=ps
      ? `${notice}Oyun paketi MediaFire üzerinden indirilecek ve Animus Emu klasörüne hazırlanacak. ${dry.created_files} dosya oluşturulacak, ${dry.changed_files} dosya değiştirilecek. Devam edilsin mi?`
      : `${notice}${dry.created_files} dosya oluşturulacak, ${dry.changed_files} dosya değiştirilecek ve ${dry.backup_files} yedek alınacak. Devam edilsin mi?`;

    if(!confirm(question))return;

    const download=await api.downloadToken(id);
    progress({stage:"download",percent:1,message:ps?"Oyun paketi indirmesi hazırlanıyor":"Yama indirmesi hazırlanıyor"});

    try{
      await patchService.install(manifest,root,download.url,false);
    }catch(error){
      const message=error instanceof Error?error.message:String(error);
      if(!/Önceki yamanın dosyaları değişmiş|yedeği bulunamadı/i.test(message))throw error;
      if(!confirm(message+"\n\nZorla devam edilsin mi? Bu dosyalardaki kendi değişiklikleriniz kaybolur."))
        {progress({stage:"error",percent:0,message:"Kurulum iptal edildi."},true);return}
      const retry=await api.downloadToken(id);
      await patchService.install(manifest,root,retry.url,true);
    }

    await installations.refresh();
    await log("info","install",`Oyun #${game.id} ${ps?"Animus Emu paketi":"yaması"} kuruldu`);
    progress({stage:"complete",percent:100,message:ps?"Oyun hazır — OYNA seçeneğini kullanabilirsin":"Kurulum tamamlandı"});
    setTimeout(()=>{closeModal();refreshCurrentView()},900);
  }catch(error){progress({stage:"error",percent:0,message:patchMessage(error)},true)}
}
function progress(event:OperationProgress,bad=false){const detail=event.total_bytes?`<small>${size(event.downloaded_bytes)} / ${size(event.total_bytes)} · ${event.bytes_per_second?size(event.bytes_per_second)+"/sn":""}</small>`:"";document.querySelector("#modal-root")!.innerHTML=`<div class="modal-backdrop"><section class="operation">${bad?'<button type="button" class="modal-close operation-close" aria-label="Kapat" title="Kapat">×</button>':""}<div class="spinner ${bad?"bad":""}"></div><h2>${escapeHtml(event.message)}</h2>${detail}<div class="operation-progress"><i style="width:${event.percent}%"></i></div><b>%${event.percent}</b></section></div>`;document.querySelector(".modal-close")?.addEventListener("click",closeModal)}
function patchMessage(error:unknown){const value=error instanceof Error?error.message:String(error);if(/sha-?256|hash|bütünlük|integrity/i.test(value))return "İndirilen yama dosyasının bütünlüğü doğrulanamadı.";if(/http|network|fetch|connection/i.test(value))return "Yama sunucusuna bağlanılamadı.";if(/unsafe|güvenli olmayan|traversal/i.test(value))return "Yama güvenlik denetiminden geçemedi.";return value.replace(/^I\/O hatası:\s*/,"Dosya işlemi tamamlanamadı: ")}

async function showBackups(){try{const backups=await patchService.backups() as {id:string;game_name:string;version:string;created_at:string;size_bytes:number;active:boolean}[];document.querySelector("#modal-root")!.innerHTML=`<div class="modal-backdrop"><section class="backups"><button class="modal-close">×</button><h2>Yedekler</h2><p class="muted">Aktif kurulumun yedeği silinemez; yamayı kaldırmak için gereklidir.</p>${backups.length?backups.map(item=>`<article><div><b>${escapeHtml(item.game_name)}</b><small>${escapeHtml(item.version)} · ${date(item.created_at)} · ${size(item.size_bytes)}${item.active?" · Aktif kurulum":""}</small></div><button class="danger clean-backup" data-id="${escapeHtml(item.id)}" ${item.active?"disabled":""}>Temizle</button></article>`).join(""):'<div class="empty-catalog">Henüz yedek bulunmuyor.</div>'}<div class="backup-maintenance"><button class="ghost" id="prune-storage">Sahipsiz Yedekleri ve İndirme Önbelleğini Temizle</button></div></section></div>`;document.querySelector(".modal-close")!.addEventListener("click",closeModal);document.querySelectorAll<HTMLButtonElement>(".clean-backup").forEach(button=>button.onclick=async()=>{try{await patchService.cleanBackup(button.dataset.id!);button.closest("article")?.remove()}catch(error){notify(patchMessage(error),true)}});document.querySelector("#prune-storage")!.addEventListener("click",async()=>{try{const report=await patchService.pruneStorage();notify(`${report.removed_backups} yedek silindi · ${size(report.freed_bytes+report.cache_bytes)} yer açıldı`);await showBackups()}catch(error){notify(patchMessage(error),true)}})}catch(error){notify(patchMessage(error),true)}}
function renderDeveloper(){const panel=document.querySelector("#dev-panel");if(!panel)return;panel.innerHTML=state.developer?`<aside class="dev"><b>DEVELOPER MODE</b><button id="dev-close">×</button><pre>${escapeHtml(JSON.stringify({user:state.user?.email,selected:state.selected?.id,games:state.games.length},null,2))}</pre></aside>`:"";document.querySelector("#dev-close")?.addEventListener("click",()=>{state.developer=false;renderDeveloper()})}
function closeModal(){document.querySelector("#modal-root")!.innerHTML=""}
function notify(text:string,bad=false){const notice=document.createElement("div");notice.className="notice "+(bad?"bad":"");notice.textContent=text;document.body.append(notice);setTimeout(()=>notice.remove(),4500)}

async function loadConfig(){try{state.config=await api.config();cache.saveConfig(state.config)}catch{state.config=fallbackConfig()}}

/** Loader kendi sürümünü Tauri paket bilgisinden okur; yama sürüm kapısı buna dayanır. */
async function loadLoaderVersion(){try{state.loaderVersion=await patchService.loaderVersion()}catch{state.loaderVersion="0.0.0"}}

/** Loader güncellemesi eskiden hiç kontrol edilmiyordu; selfUpdate.ts ölü koddu. */
async function checkLoaderUpdate(){
  try{
    const update=await checkForLoaderUpdate(false);
    if(!update)return;
    state.update=update;
    await log("info","update",`Yeni loader sürümü bulundu: ${update.version}`);
    if(document.querySelector(".shell"))shell();
  }catch(error){await log("warning","update","Loader güncelleme kontrolü başarısız: "+(error instanceof Error?error.message:"bilinmeyen hata"))}
}

async function applyLoaderUpdate(){
  if(!state.update)return;
  if(!confirm(`Loader ${state.update.version} sürümüne güncellenecek ve uygulama yeniden başlatılacak. Devam edilsin mi?`))return;
  try{progress({stage:"update",percent:10,message:"Loader güncellemesi indiriliyor"});await checkForLoaderUpdate(true)}
  catch(error){progress({stage:"error",percent:0,message:patchMessage(error)},true)}
}

/** Yedeği kaybolmuş kurulumlar sessizce bırakılmaz; kullanıcı uyarılır. */
function warnBrokenInstallations(){
  const broken=installations.brokenBackups();
  if(broken.length)notify(`${broken.length} kurulumun yedeği bulunamıyor. Bu oyunlarda "Yamayı Kaldır" orijinal dosyaları geri getiremeyebilir.`,true);
}

async function boot(){
  await loadConfig();
  await installations.refresh();
  await loadGames();
  shell();
  patchService.onProgress(event=>progress(event)).catch(()=>{});
  warnBrokenInstallations();
  void checkLoaderUpdate();
}

async function start(){
  await api.initialize();
  await loadLoaderVersion();
  await loadConfig();
  if(api.hasToken()){
    try{state.user=await api.me();await boot()}
    catch(error){loginView(error instanceof Error?error.message:"Sunucuya bağlanılamadı.",true)}
  }else loginView();
}
api.onUnauthorized(message=>{stopChatPolling();state.user=null;state.selected=null;state.games=[];loginView(message,true)});
start().catch(error=>loginView(error instanceof Error?error.message:"Uygulama başlatılamadı.",true));
