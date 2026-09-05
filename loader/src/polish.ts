import {api} from "./api/client";

const PROFESSIONAL_NAME="Animus Profesyonel Çeviri";

function setText(el:HTMLElement|null,text:string){
  if(el&&el.textContent!==text)el.textContent=text;
}

function replaceVisibleCopy(root:ParentNode=document){
  root.querySelectorAll<HTMLElement>(".login-visual .overline").forEach(el=>setText(el,"ANIMUS PROFESYONEL ÇEVİRİ"));
  root.querySelectorAll<HTMLElement>(".login-visual h1").forEach(el=>setText(el,"Profesyonel Türkçe oyun deneyimi."));
  root.querySelectorAll<HTMLElement>(".mobile-logo").forEach(el=>setText(el,PROFESSIONAL_NAME));
  root.querySelectorAll<HTMLElement>(".startup-splash-name").forEach(el=>setText(el,PROFESSIONAL_NAME));

  root.querySelectorAll<HTMLElement>(".brand.nav-button").forEach(el=>{
    if(el.dataset.animusProfessionalBrand==="1")return;
    const logo=el.querySelector<HTMLElement>(".brand-logo");
    [...el.childNodes].forEach(node=>{if(node!==logo)node.remove()});
    el.append(document.createTextNode(PROFESSIONAL_NAME));
    el.dataset.animusProfessionalBrand="1";
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

let markingDevice=false;
async function markDevice(){
  if(markingDevice||!document.querySelector(".shell")||document.querySelector(".animus-device-badge"))return;
  markingDevice=true;
  try{
    const device=await api.currentDevice();
    if(!device)return;
    const card=document.querySelector<HTMLElement>(".profile-card");
    if(!card||card.querySelector(".animus-device-badge"))return;
    const badge=document.createElement("div");
    badge.className="animus-device-badge";
    badge.textContent="✓ Cihaz doğrulandı";
    badge.title=device.device_name;
    card.append(badge);
  }catch{/* Oturum akışı kendi hata yönetimini yapar. */}
  finally{markingDevice=false}
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
