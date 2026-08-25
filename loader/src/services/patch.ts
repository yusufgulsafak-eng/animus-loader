import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import {listen} from "@tauri-apps/api/event";
import type {DryRun,InstallationSummary,OperationProgress,PruneReport,UninstallReport} from "../types";
export const patchService={
  chooseGameRoot:async(requiredFiles:string[])=>{const selected=await open({directory:true,multiple:false,title:"Oyun klasörünü seçin"});if(!selected)return null;await invoke("validate_game_root",{gameRoot:selected,requiredFiles});return selected as string},
  detectGame:(steamAppId:string|undefined,requiredFiles:string[])=>invoke<string|null>("detect_game",{steamAppId:steamAppId||null,requiredFiles}),
  dryRun:(manifest:unknown,gameRoot:string)=>invoke<DryRun>("dry_run_patch",{manifest,gameRoot}),
  install:(manifest:unknown,gameRoot:string,archiveUrl:string,force=false)=>invoke("install_patch",{manifest,gameRoot,archiveUrl,force}),
  uninstall:(gameId:number,gameRoot:string,force=false)=>invoke<UninstallReport>("uninstall_patch",{gameId,gameRoot,force}),
  listInstallations:()=>invoke<InstallationSummary[]>("list_installations"),
  loaderVersion:()=>invoke<string>("loader_version"),
  pruneStorage:()=>invoke<PruneReport>("prune_storage"),
  openExternal:(url:string)=>invoke("open_external",{url}),
  verify:(gameId:number,gameRoot:string)=>invoke("verify_installation",{gameId,gameRoot}),
  backups:()=>invoke("list_backups"),
  cleanBackup:(id:string)=>invoke("clean_backup",{backupId:id}),
  onProgress:(callback:(event:OperationProgress)=>void)=>listen<OperationProgress>("patch-progress",event=>callback(event.payload))
};
