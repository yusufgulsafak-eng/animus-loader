import type {Game,LoaderConfig,LoaderUpdateInfo,User} from "../types";
import {InstallationRegistry} from "../services/installations";
import {patchService} from "../services/patch";

export const state:{user:User|null;config:LoaderConfig|null;games:Game[];selected:Game|null;filter:string;query:string;developer:boolean;loaderVersion:string;update:LoaderUpdateInfo|null}={user:null,config:null,games:[],selected:null,filter:"all",query:"",developer:false,loaderVersion:"0.0.0",update:null};

/** Kurulu yamaların tek doğruluk kaynağı; localStorage değil Rust journal'ı. */
export const installations=new InstallationRegistry(()=>patchService.listInstallations());
export const cache={
  saveGames(games:Game[]){localStorage.setItem("games_cache",JSON.stringify({at:Date.now(),games}))},
  readGames():Game[]{try{return JSON.parse(localStorage.getItem("games_cache")||"{}").games||[]}catch{return[]}},
  saveConfig(config:LoaderConfig){localStorage.setItem("config_cache",JSON.stringify(config))},
  readConfig():LoaderConfig|null{try{return JSON.parse(localStorage.getItem("config_cache")||"null")}catch{return null}}
};

