import {invoke} from "@tauri-apps/api/core";

type LogLevel="debug"|"info"|"warning"|"error";
export async function log(level:LogLevel,category:string,message:string){
  try{await invoke("write_client_log",{level,category,message:message.replace(/Bearer\s+\S+/gi,"Bearer [REDACTED]")})}catch{/* Logging must never interrupt the user flow. */}
}
