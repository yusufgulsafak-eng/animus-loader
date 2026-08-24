import {describe,expect,it,vi} from "vitest";
import {AuthSession} from "./session";
import type {TokenStore} from "./token-store";

function store(saved:string|null=null):TokenStore&{save:ReturnType<typeof vi.fn>;clear:ReturnType<typeof vi.fn>}{return{load:vi.fn(async()=>saved),save:vi.fn(async()=>{}),clear:vi.fn(async()=>{})}}
describe("secure auth session",()=>{
  it("Credential Manager tokenını restore eder",async()=>{const s=new AuthSession(store("restored-token"));await s.initialize();expect(s.bearer()).toBe("restored-token")});
  it("logout tokenı bellekten ve güvenli depodan temizler",async()=>{const backend=store();const s=new AuthSession(backend);await s.accept("valid-token",true);await s.clear();expect(s.hasToken()).toBe(false);expect(backend.clear).toHaveBeenCalledOnce()});
  it("401 oturumu temizler ve UI handlerını çağırır",async()=>{const backend=store();const s=new AuthSession(backend);const handler=vi.fn();s.onExpired(handler);await s.accept("expired-token",false);await s.expire();expect(s.hasToken()).toBe(false);expect(handler).toHaveBeenCalledWith("Oturumunuz sona erdi. Lütfen tekrar giriş yapın.")});
});
