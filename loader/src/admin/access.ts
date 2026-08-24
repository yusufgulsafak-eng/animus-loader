import type {User} from "../types";

export function canManage(user:User|null|undefined):boolean{
  return user?.role==="admin"||user?.role==="super_admin";
}
