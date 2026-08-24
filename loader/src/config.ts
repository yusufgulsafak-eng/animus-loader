const raw=import.meta.env.VITE_API_BASE_URL||import.meta.env.VITE_API_URL||(import.meta.env.DEV?"http://127.0.0.1:8080":"");
if(!raw)throw new Error("Production build için VITE_API_BASE_URL tanımlanmalıdır.");
const configured=raw.replace(/\/$/,"");
if(import.meta.env.PROD&&!configured.startsWith("https://"))throw new Error("Production API URL HTTPS olmalıdır.");
export const API_BASE_URL=(configured.endsWith("/api")?configured:configured+"/api");
export const WEB_BASE_URL=API_BASE_URL.replace(/\/api$/,"");
