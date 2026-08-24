import {secureTokenStore,type TokenStore} from "./token-store";

export class AuthSession {
  private token:string|null=null;
  private initialization:Promise<void>|null=null;
  private expiredHandlers=new Set<(message:string)=>void>();
  constructor(private readonly store:TokenStore=secureTokenStore){}
  initialize():Promise<void>{return this.initialization??=(async()=>{try{this.token=await this.store.load()}catch{this.token=null}})()}
  bearer():string|null{return this.token}
  hasToken():boolean{return Boolean(this.token)}
  async accept(token:string,persist=true):Promise<void>{
    this.token=token;
    try{
      if(persist)await this.store.save(token);else await this.store.clear();
    }catch(error){
      this.token=null;
      throw error;
    }
  }
  async clear():Promise<void>{this.token=null;try{await this.store.clear()}catch{/* Bellek oturumu yine de kapatılır. */}}
  async expire(message="Oturumunuz sona erdi. Lütfen tekrar giriş yapın."):Promise<void>{await this.clear();this.expiredHandlers.forEach(handler=>handler(message))}
  onExpired(handler:(message:string)=>void):()=>void{this.expiredHandlers.add(handler);return()=>this.expiredHandlers.delete(handler)}
}

export const authSession=new AuthSession();
