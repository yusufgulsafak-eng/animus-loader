import {describe,expect,it} from "vitest";
import {filterCatalog} from "./catalog";
import type {Game} from "./types";

const games=[
  {id:1,name:"Assassin's Creed Mirage",slug:"mirage",access_type:"free",translation_percent:100,patch_version:"1.2.0",categories:[]},
  {id:2,name:"Resident Evil Village",slug:"village",access_type:"premium",translation_percent:80,patch_version:"2.0.0",categories:[]},
  {id:3,name:"Silent Hill 3",slug:"silent-hill-3",access_type:"free",translation_percent:0,patch_version:null,categories:[]},
] satisfies Game[];
const installed=new Map([[1,"1.0.0"],[2,"2.0.0"]]);
const version=(id:number)=>installed.get(id)??null;

describe("generic catalog filters",()=>{
  it("searches game names",()=>expect(filterCatalog(games,"village","all",version).map(g=>g.id)).toEqual([2]));
  it("filters free and premium",()=>{
    expect(filterCatalog(games,"","free",version).map(g=>g.id)).toEqual([1,3]);
    expect(filterCatalog(games,"","premium",version).map(g=>g.id)).toEqual([2]);
  });
  it("detects installed games and updates",()=>{
    expect(filterCatalog(games,"","installed",version).map(g=>g.id)).toEqual([1,2]);
    expect(filterCatalog(games,"","update",version).map(g=>g.id)).toEqual([1]);
  });
});

