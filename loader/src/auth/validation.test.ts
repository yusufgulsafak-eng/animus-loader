import {describe,expect,it} from "vitest";
import {validateRegistration} from "./validation";
const valid={displayName:"Animus User",email:"user@example.test",password:"StrongPassword42",passwordConfirm:"StrongPassword42"};
describe("registration validation",()=>{
  it("geçerli formu kabul eder",()=>expect(validateRegistration(valid)).toBeNull());
  it("geçersiz e-posta ve zayıf şifreyi reddeder",()=>{expect(validateRegistration({...valid,email:"bad"})).toContain("e-posta");expect(validateRegistration({...valid,password:"weak",passwordConfirm:"weak"})).toContain("12")});
  it("eşleşmeyen şifreleri reddeder",()=>expect(validateRegistration({...valid,passwordConfirm:"Different42Password"})).toBe("Şifreler eşleşmiyor."));
});
