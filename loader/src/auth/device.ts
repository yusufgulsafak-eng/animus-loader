const DEVICE_KEY="animus_device_uuid_v1";
let memoryDeviceId="";

function uuidV4():string{
  if(typeof crypto!=="undefined"&&typeof crypto.randomUUID==="function")return crypto.randomUUID();
  const bytes=new Uint8Array(16);crypto.getRandomValues(bytes);bytes[6]=(bytes[6]&0x0f)|0x40;bytes[8]=(bytes[8]&0x3f)|0x80;
  const hex=[...bytes].map(value=>value.toString(16).padStart(2,"0"));
  return `${hex.slice(0,4).join("")}-${hex.slice(4,6).join("")}-${hex.slice(6,8).join("")}-${hex.slice(8,10).join("")}-${hex.slice(10,16).join("")}`;
}

function validUuid(value:string):boolean{
  return /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(value);
}

export function deviceId():string{
  let value="";
  try{value=localStorage.getItem(DEVICE_KEY)?.trim().toLowerCase()||""}catch{}
  if(!validUuid(value))value=memoryDeviceId;
  if(!validUuid(value)){
    value=uuidV4().toLowerCase();
    memoryDeviceId=value;
    try{localStorage.setItem(DEVICE_KEY,value)}catch{}
  }else{
    memoryDeviceId=value;
  }
  return value;
}

export function deviceName():string{
  const platform=(navigator as Navigator&{userAgentData?:{platform?:string}}).userAgentData?.platform||navigator.platform||"Windows";
  return `Animus ${platform} PC`.slice(0,190);
}

export function clearDeviceIdentity():void{
  memoryDeviceId="";
  try{localStorage.removeItem(DEVICE_KEY)}catch{}
}
