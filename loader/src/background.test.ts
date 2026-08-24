import {describe,expect,it,vi} from "vitest";
import {activateBackgroundVideo,clearActiveBackgroundVideo,normalizeBackground} from "./background";

const fakeVideo=()=>({pause:vi.fn(),load:vi.fn(),removeAttribute:vi.fn()});

describe("branding background",()=>{
  it("unknown ve eksik medya ayarlarını güvenli varsayılana çevirir",()=>{
    expect(normalizeBackground({type:"script",video_url:"bad"}).type).toBe("default");
    expect(normalizeBackground({type:"image",image_url:null}).type).toBe("default");
  });
  it("eksik videoda fallback resmi kullanır ve overlay değerini sınırlar",()=>{
    expect(normalizeBackground({type:"video",fallback_url:"/fallback.webp",overlay:150})).toMatchObject({type:"image",imageUrl:"/fallback.webp",overlay:100});
  });
  it("login videosundan library videosuna geçerken eski kaynağı temizler",()=>{
    const login=fakeVideo(),library=fakeVideo();activateBackgroundVideo(login);activateBackgroundVideo(library);
    expect(login.pause).toHaveBeenCalledOnce();expect(login.removeAttribute).toHaveBeenCalledWith("src");expect(login.load).toHaveBeenCalledOnce();
    activateBackgroundVideo(library);expect(library.pause).not.toHaveBeenCalled();clearActiveBackgroundVideo();expect(library.pause).toHaveBeenCalledOnce();
  });
});
