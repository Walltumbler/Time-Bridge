const api=globalThis.browser??chrome;
const HOST="app.timebridge.bridge";
const extensionError=(text)=>({error:String(text||"Timebridge bridge unavailable")});
async function invokeNative(origin,request,credential){
  let response;
  try{response=await api.runtime.sendNativeMessage(HOST,{origin,request,...(credential?{credential}:{})});}
  catch{return extensionError("Open Timebridge Desktop. The local browser helper is unavailable.");}
  if(response?.error)return extensionError(response.error);
  if(request.op==="permissions.poll"&&response?.status==="approved"){
    if(!response.credential)return extensionError("Timebridge did not return a pairing credential.");
    await api.storage.local.set({[`credential:${origin}`]:response.credential});
    return {status:"approved"};
  }
  if(request.op==="permissions.poll"&&response?.status==="denied")return {status:"denied"};
  return response;
}
async function handle(message,sender){
  if(!message||message.type!=="TIMEBRIDGE_REQUEST"||sender.frameId!==0||!sender.tab?.id||!sender.tab.url)return extensionError("Only the connected top-level website can use Timebridge.");
  let origin;try{const u=new URL(sender.tab.url);origin=u.origin;if(u.protocol!=="https:"&&!(u.protocol==="http:"&&["localhost","127.0.0.1"].includes(u.hostname)))throw 0;}catch{return extensionError("Only HTTPS websites and localhost development sites are allowed.");}
  const request=message.request;
  if(!request||typeof request!=="object"||typeof request.op!=="string")return extensionError("Invalid Timebridge request.");
  if(request.op==="detect")return invokeNative(origin,request);
  if(request.op==="permissions.request"){
    if(typeof request.name!=="string"||typeof request.pollSecret!=="string")return extensionError("Invalid pairing request.");
    return invokeNative(origin,request);
  }
  if(request.op==="permissions.poll"){
    if(typeof request.id!=="string"||typeof request.pollSecret!=="string")return extensionError("Invalid pairing poll.");
    return invokeNative(origin,request);
  }
  const found=await api.storage.local.get(`credential:${origin}`);const credential=found[`credential:${origin}`];
  if(typeof credential!=="string")return extensionError("Request website permission in Timebridge Desktop first.");
  return invokeNative(origin,request,credential);
}
api.runtime.onMessage.addListener((message,sender,sendResponse)=>{
  if(message?.type!=="TIMEBRIDGE_REQUEST"&&message?.type!=="TIMEBRIDGE_STATUS")return undefined;
  const work=message.type==="TIMEBRIDGE_STATUS"?handle({type:"TIMEBRIDGE_REQUEST",request:{op:"detect"}},sender):handle(message,sender);
  work.then(sendResponse).catch(e=>sendResponse(extensionError(e?.message)));return true;
});
