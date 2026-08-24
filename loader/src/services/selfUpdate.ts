import {check} from "@tauri-apps/plugin-updater";
export async function checkForLoaderUpdate(install=false){
  const update=await check();
  if(!update)return null;
  const info={version:update.version,currentVersion:update.currentVersion,body:update.body,date:update.date};
  if(install)await update.downloadAndInstall();
  return info;
}

