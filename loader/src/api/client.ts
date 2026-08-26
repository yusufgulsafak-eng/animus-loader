import {API_BASE_URL} from "../config";
import {log} from "../services/logger";
import {authSession} from "../auth/session";
import type {ApiEnvelope,Game,LoaderConfig,User} from "../types";
import type {AdminPanelData} from "../admin/types";

export type CommunityAnnouncement={
  id:number;
  title:string;
  body:string;
  audience:"all"|"free"|"premium"|"tester"|"admin";
  starts_at?:string|null;
  ends_at?:string|null;
  created_at:string;
};

export type ChatMessage={
  id:number;
  user_id:number;
  body:string;
  created_at:string;
  display_name:string;
  role:"user"|"tester"|"admin"|"super_admin";
};

export class ApiError extends Error{
  constructor(message:string,public status=0){super(message);this.name="ApiError";}
}

function userMessage(error:unknown,status=0){
  if(error instanceof DOMException&&error.name==="AbortError")return "Sunucu yanıt vermedi. Lütfen tekrar deneyin.";
  if(error instanceof TypeError||status===0)return "Animus sunucusuna ulaşılamıyor. İnternet bağlantınızı kontrol edin.";
  if(status===401)return "Oturumunuz sona erdi. Lütfen yeniden giriş yapın.";
  if(status===403)return "Bu işlem için yetkiniz bulunmuyor.";
  if(status===429)return "Çok hızlı işlem yapıyorsun. Kısa süre sonra tekrar dene.";
  if(status>=500)return "Sunucu tarafında bir hata oluştu.";
  return error instanceof Error?error.message:"İşlem tamamlanamadı.";
}

async function request<T>(path:string,init:RequestInit={}):Promise<T>{
  await initialize();
  const headers=new Headers(init.headers);headers.set("Accept","application/json");
  if(init.body&&!(init.body instanceof FormData)&&!headers.has("Content-Type"))headers.set("Content-Type","application/json");
  if(authSession.bearer())headers.set("Authorization","Bearer "+authSession.bearer());
  const controller=new AbortController();const timer=window.setTimeout(()=>controller.abort(),20000);
  try{
    const response=await fetch(API_BASE_URL+path,{...init,headers,signal:controller.signal});
    let body:ApiEnvelope<T>|null=null;try{body=await response.json() as ApiEnvelope<T>}catch{throw new ApiError(response.ok?"Sunucu geçersiz bir yanıt döndürdü.":"Sunucu tarafında bir hata oluştu.",response.status)}
    if(!response.ok||!body.ok)throw new ApiError(body.error?.message||userMessage(null,response.status),response.status);
    return body.data;
  }catch(error){
    const status=error instanceof ApiError?error.status:0;const message=userMessage(error,status);if(status===401)await authSession.expire(message);
    await log("error","api",`${init.method||"GET"} ${path} başarısız (HTTP ${status||"network"})`);
    throw new ApiError(message,status);
  }finally{window.clearTimeout(timer)}
}

async function initialize(){localStorage.removeItem("loader_token");await authSession.initialize()}

export const api={
  initialize,
  async login(email:string,password:string,remember=true){
    const result=await request<{user:User;token:string}>("/auth/login",{method:"POST",body:JSON.stringify({email,password})});
    await authSession.accept(result.token,remember);
    await log("info","login","Kullanıcı girişi başarılı");
    return result.user;
  },
  async register(displayName:string,email:string,password:string){const result=await request<{user:User;token:string;message:string}>("/auth/register",{method:"POST",body:JSON.stringify({display_name:displayName,email,password})});await authSession.accept(result.token,true);await log("info","login","Kullanıcı kaydı ve güvenli otomatik giriş başarılı");return result.user},
  me:()=>request<User>("/auth/me"),

  updateProfile:(displayName:string)=>request<User>("/profile",{method:"PATCH",body:JSON.stringify({display_name:displayName})}),
  changePassword:(currentPassword:string,newPassword:string)=>request<{message:string}>("/profile/password",{method:"POST",body:JSON.stringify({current_password:currentPassword,new_password:newPassword})}),
  uploadAvatar:(file:File)=>{
    const form=new FormData();
    form.set("avatar",file);
    return request<User>("/profile/avatar",{method:"POST",body:form});
  },
  removeAvatar:()=>request<User>("/profile/avatar",{method:"DELETE"}),

  announcements:()=>request<CommunityAnnouncement[]>("/announcements"),
  chatMessages:()=>request<ChatMessage[]>("/chat/messages"),
  sendChat:(body:string)=>request<ChatMessage>("/chat/messages",{method:"POST",body:JSON.stringify({body})}),
  deleteChat:(id:number)=>request<null>("/chat/messages/"+id,{method:"DELETE"}),

  games:(q="")=>request<Game[]>("/games"+(q?"?q="+encodeURIComponent(q):"")),
  game:(id:number)=>request<Game>("/games/"+id),
  patch:(id:number)=>request<Record<string,unknown>>("/games/"+id+"/patch"),
  manifest:(id:number)=>request<Record<string,unknown>>("/patches/"+id+"/manifest"),
  downloadToken:(id:number)=>request<{url:string;expires_in:number}>("/patches/"+id+"/download-token",{method:"POST"}),
  config:()=>request<LoaderConfig>("/loader/config"),
  latest:(channel="stable")=>request<Record<string,unknown>|null>("/loader/latest?channel="+channel),
  adminPanel:()=>request<AdminPanelData>("/admin/panel"),
  adminAction:<T=unknown>(action:string,payload:Record<string,unknown>={})=>request<T>("/admin/action",{method:"POST",body:JSON.stringify({action,...payload})}),
  adminUpload:<T=unknown>(action:string,form:FormData)=>{form.set("action",action);return request<T>("/admin/action",{method:"POST",body:form})},
  logout:async()=>{try{if(authSession.hasToken())await request("/auth/logout",{method:"POST"})}finally{await authSession.clear();await log("info","login","Kullanıcı çıkışı tamamlandı")}},
  hasToken:()=>authSession.hasToken(),
  onUnauthorized:(handler:(message:string)=>void)=>authSession.onExpired(handler)
};
