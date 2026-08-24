import {describe,expect,it} from "vitest";
import {canManage} from "./access";
import type {User} from "../types";

const user=(role:string)=>({id:1,email:"test@example.com",display_name:"Test",role,release_channel:"stable",premium:false} as User);

describe("loader admin access",()=>{
  it("allows admin and super_admin",()=>{
    expect(canManage(user("admin"))).toBe(true);
    expect(canManage(user("super_admin"))).toBe(true);
  });
  it("denies normal users and missing sessions",()=>{
    expect(canManage(user("user"))).toBe(false);
    expect(canManage(null)).toBe(false);
  });
});
