const api=globalThis.browser??chrome;
const $=s=>document.querySelector(s);
async function start(){
  const [tab]=await api.tabs.query({active:true,currentWindow:true});
  if(!tab?.id||!tab.url){$("#origin").textContent="No active website tab";$("#message").textContent="Open an HTTPS website, then select Timebridge again.";$("#message").className="error";return;}
  let url;try{url=new URL(tab.url);}catch{url=null;}
  const allowed=url&&(url.protocol==="https:"||(url.protocol==="http:"&&["localhost","127.0.0.1"].includes(url.hostname)));
  if(!allowed){$("#origin").textContent="This page cannot connect to Timebridge";$("#message").textContent="For your safety, only HTTPS websites and localhost development sites are supported.";$("#message").className="error";return;}
  $("#origin").textContent=url.origin;$("#message").textContent="Ready to connect this tab to Timebridge Desktop.";$("#connect").disabled=false;
  $("#connect").onclick=async()=>{
    $("#connect").disabled=true;$("#message").textContent="Checking the desktop app…";$("#message").className="";
    try{
      await api.scripting.executeScript({target:{tabId:tab.id,frameIds:[0]},files:["src/content.js"]});
      const result=await api.tabs.sendMessage(tab.id,{type:"TIMEBRIDGE_STATUS"});
      if(!result?.desktop)throw new Error("Open Timebridge Desktop and try again.");
      $("#message").textContent="Requesting permission… Approve this site in Timebridge Desktop.";
      const state=await api.tabs.sendMessage(tab.id,{type:"TIMEBRIDGE_CONNECT",name:url.hostname});
      if(state?.error)throw new Error(state.error);
      if(state?.status!=="approved")throw new Error("Website access was denied in Timebridge Desktop.");
      $("#message").textContent="This site is connected. You can close this panel.";
      $("#message").className="ok";
    }catch(e){$("#message").textContent=String(e?.message||"Could not connect. Open Timebridge Desktop and try again.");$("#message").className="error";$("#connect").disabled=false;}
  };
}
void start().catch(()=>{$("#message").textContent="Could not check the current website.";$("#message").className="error";});
