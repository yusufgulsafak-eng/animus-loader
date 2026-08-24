import {invoke} from "@tauri-apps/api/core";

export interface TokenStore {load():Promise<string|null>;save(token:string):Promise<void>;clear():Promise<void>}

export const secureTokenStore:TokenStore={
  load:()=>invoke<string|null>("load_access_token"),
  save:(token:string)=>invoke("save_access_token",{token}),
  clear:()=>invoke("clear_access_token")
};
