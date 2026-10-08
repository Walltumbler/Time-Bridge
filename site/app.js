import { createTimebridge, iconFromFile } from './sdk/index.js';
const $ = selector => document.querySelector(selector);
let clock = createTimebridge({appName:'Timebridge Demo'}), selectedIcon;
const status = (selector, text, kind='') => { const el=$(selector);el.textContent=text;el.className=`status ${kind}`; };
$('#site-origin').textContent=location.origin;
function chooseIcon(kind){
  if(kind==='none') selectedIcon=undefined;
  else { const canvas=document.createElement('canvas');canvas.width=canvas.height=64;const ctx=canvas.getContext('2d');ctx.fillStyle=kind==='cup'?'#edf3ff':'#eaf7ef';ctx.fillRect(0,0,64,64);ctx.font='40px Segoe UI Emoji';ctx.textAlign='center';ctx.textBaseline='middle';ctx.fillText(kind==='cup'?'☕':'🌿',32,34);selectedIcon=canvas.toDataURL('image/png'); }
  $('#icon-preview').hidden=!selectedIcon;if(selectedIcon)$('#icon-preview').src=selectedIcon;
  document.querySelectorAll('[data-icon]').forEach(b=>{const selected=b.dataset.icon===kind;b.classList.toggle('selected',selected);b.setAttribute('aria-pressed',String(selected));});
}
chooseIcon('cup');
document.querySelectorAll('[data-icon]').forEach(b=>b.onclick=()=>chooseIcon(b.dataset.icon));
$('#icon-file').onchange=async()=>{try{const file=$('#icon-file').files[0];if(!file)return;selectedIcon=await iconFromFile(file);$('#icon-preview').src=selectedIcon;$('#icon-preview').hidden=false;document.querySelectorAll('[data-icon]').forEach(b=>{b.classList.remove('selected');b.setAttribute('aria-pressed','false');});status('#result','Icon ready. Create your test alarm.');}catch(e){status('#result',e.message,'error');}};
$('#transport').onchange=()=>{clock.dispose();clock=createTimebridge({appName:'Timebridge Demo',transport:$('#transport').value});$('#create').disabled=true;status('#pairing','Connection method changed. Connect again.');};
$('#detect').onclick=async()=>{const button=$('#detect');button.disabled=true;status('#connection','Looking for Timebridge…');try{const result=await clock.connect();status('#connection',result.desktop?`Desktop ${result.version} is ready · ${result.transport==='extension'?'browser extension':'direct local connection'}.`:'Desktop not found. Open it, or connect the browser extension below.',result.desktop?'ok':'error');}catch(e){status('#connection',e.message,'error');}finally{button.disabled=false;}};
$('#connect').onclick=async()=>{const button=$('#connect');button.disabled=true;$('#transport').disabled=true;status('#pairing','Approve the connection in Timebridge Desktop, then return here.');try{await clock.requestPermission();$('#create').disabled=false;status('#pairing','Connected. This page can create its own alarms.','ok');status('#result','Ready to create a 10-second alarm.');}catch(e){status('#pairing',e.message,'error');}finally{button.disabled=false;$('#transport').disabled=false;}};
$('#create').onclick=async()=>{const button=$('#create');button.disabled=true;try{const item=await clock.createCountdown({durationSeconds:10,title:$('#title').value.trim()||undefined,icon:selectedIcon});status('#result',`“${item.title}” was created in Desktop. It will ring in 10 seconds. You can close this page now.`,'ok');}catch(e){status('#result',e.message,'error');}finally{button.disabled=false;}};
document.querySelectorAll('[data-copy]').forEach(b=>b.onclick=async()=>{try{await navigator.clipboard.writeText(b.dataset.copy);b.textContent='Copied — paste into the address bar';}catch{b.textContent=b.dataset.copy;}});
fetch('./release.config.json').then(r=>r.json()).then(config=>{
  for(const [browser,key] of [['chrome','chromeStoreUrl'],['firefox','firefoxStoreUrl']])if(config[key]){const url=new URL(config[key]);if(url.protocol!=='https:')continue;$(`#${browser}-store`).href=url.href;$(`#${browser}-store`).hidden=false;$(`#${browser}-note`).textContent='Install from the browser’s official add-on store, then return here.';}
}).catch(()=>{});
