import type {Game,LoaderConfig,User} from "../types";
export const state:{user:User|null;config:LoaderConfig|null;games:Game[];selected:Game|null;filter:string;query:string;developer:boolean}={user:null,config:null,games:[],selected:null,filter:"all",query:"",developer:false};
export const cache={
  saveGames(games:Game[]){localStorage.setItem("games_cache",JSON.stringify({at:Date.now(),games}))},
  readGames():Game[]{try{return JSON.parse(localStorage.getItem("games_cache")||"{}").games||[]}catch{return[]}},
  saveConfig(config:LoaderConfig){localStorage.setItem("config_cache",JSON.stringify(config))},
  readConfig():LoaderConfig|null{try{return JSON.parse(localStorage.getItem("config_cache")||"null")}catch{return null}}
};

