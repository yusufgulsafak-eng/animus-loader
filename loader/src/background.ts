import type {BackgroundConfig,BackgroundType} from "./types";

export interface NormalizedBackground {type:BackgroundType;imageUrl:string;videoUrl:string;fallbackUrl:string;overlay:number;version:string}
type ManagedVideo = Pick<HTMLVideoElement,"pause"|"load"|"removeAttribute">;
let activeVideo:ManagedVideo|null=null;

export function normalizeBackground(input:BackgroundConfig|undefined,defaultOverlay=55):NormalizedBackground{
  const requested=input?.type;
  const type:BackgroundType=requested==="image"||requested==="video"||requested==="default"?requested:"default";
  const imageUrl=typeof input?.image_url==="string"?input.image_url:"";
  const videoUrl=typeof input?.video_url==="string"?input.video_url:"";
  const fallbackUrl=typeof input?.fallback_url==="string"?input.fallback_url:"";
  const rawOverlay=Number(input?.overlay??defaultOverlay);
  const overlay=Number.isFinite(rawOverlay)?Math.max(0,Math.min(100,rawOverlay)):defaultOverlay;
  if(type==="image"&&!imageUrl)return {type:"default",imageUrl:"",videoUrl:"",fallbackUrl:"",overlay,version:""};
  if(type==="video"&&!videoUrl){
    if(fallbackUrl)return {type:"image",imageUrl:fallbackUrl,videoUrl:"",fallbackUrl:"",overlay,version:String(input?.version||"")};
    return {type:"default",imageUrl:"",videoUrl:"",fallbackUrl:"",overlay,version:""};
  }
  return {type,imageUrl,videoUrl,fallbackUrl,overlay,version:String(input?.version||"")};
}

export function activateBackgroundVideo(video:ManagedVideo|null):void{
  if(activeVideo===video)return;
  clearActiveBackgroundVideo();
  activeVideo=video;
}

export function clearActiveBackgroundVideo():void{
  if(!activeVideo)return;
  activeVideo.pause();
  activeVideo.removeAttribute("src");
  activeVideo.load();
  activeVideo=null;
}

function preloadVideo(url:string):void{
  if(!url||document.head.querySelector(`link[data-animus-video-preload="${CSS.escape(url)}"]`))return;
  const link=document.createElement("link");
  link.rel="preload";
  link.as="video";
  link.href=url;
  link.dataset.animusVideoPreload=url;
  document.head.append(link);
}

export function mountBackgroundMedia(container:HTMLElement,input:BackgroundConfig|undefined,resolveUrl:(url:string)=>string,defaultOverlay=55):void{
  clearActiveBackgroundVideo();
  container.querySelector(":scope > .background-media-layer")?.remove();
  const config=normalizeBackground(input,defaultOverlay);
  if(config.type==="default")return;

  const layer=document.createElement("div");
  layer.className="background-media-layer";
  layer.dataset.mediaVersion=config.version;

  const overlay=document.createElement("div");
  overlay.className="background-media-overlay";
  overlay.style.opacity=String(config.overlay/100);

  const fail=()=>{
    clearActiveBackgroundVideo();
    layer.querySelector("video,img")?.remove();
    if(config.fallbackUrl){
      const fallback=document.createElement("img");
      fallback.src=resolveUrl(config.fallbackUrl);
      fallback.alt="";
      fallback.decoding="async";
      fallback.addEventListener("error",()=>layer.remove(),{once:true});
      layer.prepend(fallback);
    }else layer.remove();
  };

  // Katmanı önce DOM'a koyuyoruz. Böylece fallback/poster ilk kare gelene kadar
  // boş ekran yerine anında görünür ve video download'u render ile paralel başlar.
  layer.append(overlay);
  container.prepend(layer);

  if(config.type==="image"){
    const image=document.createElement("img");
    image.src=resolveUrl(config.imageUrl);
    image.alt="";
    image.loading="eager";
    image.decoding="async";
    image.addEventListener("error",()=>layer.remove(),{once:true});
    layer.prepend(image);
    return;
  }

  const videoUrl=resolveUrl(config.videoUrl);
  preloadVideo(videoUrl);
  const video=document.createElement("video");
  video.muted=true;
  video.defaultMuted=true;
  video.loop=true;
  video.autoplay=true;
  video.playsInline=true;
  video.preload="auto";
  video.disablePictureInPicture=true;
  video.setAttribute("playsinline","");
  video.setAttribute("webkit-playsinline","");
  if(config.fallbackUrl)video.poster=resolveUrl(config.fallbackUrl);
  video.addEventListener("loadeddata",()=>layer.classList.add("media-ready"),{once:true});
  video.addEventListener("canplay",()=>layer.classList.add("media-ready"),{once:true});
  video.addEventListener("error",fail,{once:true});
  layer.prepend(video);
  activateBackgroundVideo(video);
  video.src=videoUrl;
  video.load();
  void video.play().catch(()=>{/* muted autoplay WebView hazır olduğunda yeniden denenir */});
}
