(() => {
  if(window.__timebridgeContentInstalled)return;
  Object.defineProperty(window,"__timebridgeContentInstalled",{value:true,configurable:false});
  const api=globalThis.browser??chrome;
  const source="timebridge-sdk";
  let pairingTask=null;
  async function connect(name){
    const call=request=>api.runtime.sendMessage({type:"TIMEBRIDGE_REQUEST",request});
    const existing=await call({op:"permissions.status"});
    if(existing?.approved)return {status:"approved"};
    const pollSecret=[...crypto.getRandomValues(new Uint8Array(32))].map(b=>b.toString(16).padStart(2,"0")).join("");
    const pairing=await call({op:"permissions.request",name,pollSecret});
    if(pairing?.error)return pairing;
    const deadline=Date.now()+120000;
    while(Date.now()<deadline){
      const state=await call({op:"permissions.poll",id:pairing.id,pollSecret});
      if(state?.error||state?.status==="approved"||state?.status==="denied")return state;
      await new Promise(r=>setTimeout(r,1100));
    }
    return {error:"Pairing expired. Select Connect this site again."};
  }
  window.addEventListener("message",event=>{
    if(event.source!==window||event.origin!==location.origin||event.data?.source!==source||typeof event.data?.id!=="string"||!event.data.request)return;
    api.runtime.sendMessage({type:"TIMEBRIDGE_REQUEST",request:event.data.request}).then(result=>{
      window.postMessage({source:"timebridge-extension",id:event.data.id,result},location.origin);
    }).catch(error=>window.postMessage({source:"timebridge-extension",id:event.data.id,error:String(error?.message||error)},location.origin));
  });
  api.runtime.onMessage.addListener(message=>{
    if(message?.type==="TIMEBRIDGE_CONNECT"){
      // The page stays alive when switching to Desktop closes the toolbar popup.
      if(!pairingTask)pairingTask=connect(message.name).catch(e=>({error:String(e?.message||e)})).finally(()=>{pairingTask=null;});
      return pairingTask;
    }
    if(message?.type==="TIMEBRIDGE_STATUS")return api.runtime.sendMessage({type:"TIMEBRIDGE_STATUS"});
    if(message?.type==="TIMEBRIDGE_POPUP_REQUEST")return api.runtime.sendMessage({type:"TIMEBRIDGE_REQUEST",request:message.request});
    return undefined;
  });
})();
