import { defineConfig, loadEnv } from "vite";

export default defineConfig(({mode})=>{
  const env=loadEnv(mode,".","VITE_");
  if(mode==="production"){
    const apiBaseUrl=(env.VITE_API_BASE_URL||"").trim().replace(/\/$/,"");
    if(!apiBaseUrl)throw new Error("Production build için VITE_API_BASE_URL zorunludur.");
    if(apiBaseUrl!=="https://api.yusufgulsafak.com")throw new Error("VITE_API_BASE_URL https://api.yusufgulsafak.com olmalıdır.");
  }
  return {clearScreen:false,server:{port:1420,strictPort:true,watch:{ignored:["**/src-tauri/**"]}},envPrefix:["VITE_","TAURI_"],build:{target:["es2021","chrome100","safari13"],minify:"esbuild",sourcemap:false}};
});
