import type {Game} from "./types";

export type CatalogFilter="all"|"installed"|"update"|"free"|"premium";

export function filterCatalog(
  games:Game[],
  query:string,
  filter:CatalogFilter|string,
  installedVersion:(gameId:number)=>string|null,
):Game[]{
  const needle=query.trim().toLocaleLowerCase("tr");
  let result=needle?games.filter(game=>game.name.toLocaleLowerCase("tr").includes(needle)):games.slice();
  if(filter==="free"||filter==="premium")result=result.filter(game=>game.access_type===filter);
  if(filter==="installed")result=result.filter(game=>installedVersion(game.id)!==null);
  if(filter==="update")result=result.filter(game=>{const installed=installedVersion(game.id);return Boolean(installed&&game.patch_version&&installed!==game.patch_version)});
  return result;
}

