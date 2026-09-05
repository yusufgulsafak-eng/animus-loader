import {api} from "./api/client";

function replaceVisibleCopy(root:ParentNode=document){
  root.querySelectorAll<HTMLElement>(".login-visual .overline").forEach(el=>{el.textContent="ANIMUS PROFESYONEL ÇEVİRİ"});
  root.querySelectorAll<HTMLElement>(".login-visual h1").forEach(el=>{el.textContent="Profesyonel Türkçe oyun deneyimi."});
  root.querySelectorAll<HTMLElement>(".mobile-logo").forEach(el=>{el.textContent="Animus Profesyonel Çeviri"});
  root.querySelectorAll<HTMLElement>(".startup-splash-name").forEach(el=>{el.textContent="Animus Profesyonel Çeviri"});

  root.querySelectorAll<HTMLElement>(".brand.nav-button").forEach(el=>{
    const image=el.querySelector(".brand-logo")?.outerHTML||"";
    el.innerHTML=image+"Animus Profesyonel Çeviri";
  });

  root.querySelectorAll<HTMLElement>(".operation").forEach(operation=>{
    if(operation.querySelector(".animus-security-status"))return;
    const status=document.createElement("div");
    status.className="animus-security-status";
    status.innerHTML='<span><i>✓</i>Kullanıcı doğrulandı</span><span><i>✓</i>Cihaz doğrulandı</span>';
    const progress=operation.querySelector(".operation-progress");
    if(progress)operation.insertBefore(status,progress);else operation.append(status);
  });
}

async function markDevice(){
  if(!document.querySelector(".shell")||document.querySelector(".animus-device-badge"))return;
  try{
    const device=await api.currentDevice();
    if(!device)return;
    const card=document.querySelector<HTMLElement>(".profile-card");
    if(!card)return;
    const badge=document.createElement("div");
    badge.className="animus-device-badge";
    badge.textContent="✓ Cihaz doğrulandı";
    badge.title=device.device_name;
    card.append(badge);
  }catch{/* Oturum akışı kendi hata yönetimini yapar. */}
}

let scheduled=false;
function schedule(){
  if(scheduled)return;
  scheduled=true;
  queueMicrotask(()=>{
    scheduled=false;
    replaceVisibleCopy();
    void markDevice();
  });
}

new MutationObserver(schedule).observe(document.documentElement,{childList:true,subtree:true});
window.addEventListener("DOMContentLoaded",schedule,{once:true});
schedule();
