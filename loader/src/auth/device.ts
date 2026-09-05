const DEVICE_KEY="animus_device_uuid_v1";

function uuidV4():string{
  if(typeof crypto!=="undefined"&&typeof crypto.randomUUID==="function")return crypto.randomUUID();
  const bytes=new Uint8Array(16);crypto.getRandomValues(bytes);bytes[6]=(bytes[6]&0x0f)|0x40;bytes[8]=(bytes[8]&0x3f)|0x80;
  const hex=[...bytes].map(value=>value.toString(16).padStart(2,"0"));
  return `${hex.slice(0,4).join("")}-${hex.slice(4,6).join("")}-${hex.slice(6,8).join("")}-${hex.slice(8,10).join("")}-${hex.slice(10,16).join("")}`;
}

export function deviceId():string{
  let value=localStorage.getItem(DEVICE_KEY)?.trim().toLowerCase()||"";
  if(!/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(value)){
    value=uuidV4().toLowerCase();
    localStorage.setItem(DEVICE_KEY,value);
  }
  return value;
}

export function deviceName():string{
  const platform=(navigator as Navigator&{userAgentData?:{platform?:string}}).userAgentData?.platform||navigator.platform||"Windows";
  return `Animus ${platform} PC`.slice(0,190);
}

export function clearDeviceIdentity():void{
  localStorage.removeItem(DEVICE_KEY);
}
