import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,existsSync} from 'node:fs';
import vm from 'node:vm';
import {webcrypto} from 'node:crypto';
test('fresh extension detects Desktop without granting unpaired item access',async()=>{
  const listeners=[],requests=[];
  const api={runtime:{onMessage:{addListener:f=>listeners.push(f)},sendNativeMessage:async(_,payload)=>{requests.push(payload);return {desktop:true};}},storage:{local:{get:async()=>({}),set:async()=>{}}}};
  vm.runInNewContext(readFileSync('extensions/dist/chrome/src/background.js','utf8'),{chrome:api,URL});
  const send=request=>new Promise(resolve=>listeners[0]({type:'TIMEBRIDGE_REQUEST',request},{frameId:0,tab:{id:1,url:'https://example.com/'}},resolve));
  assert.equal((await send({op:'detect'})).desktop,true);
  assert.match((await send({op:'items.list'})).error,/permission/);assert.equal(requests.length,1);
});
test('pairing belongs to the page, outlives the popup, and deduplicates requests',async()=>{
  const listeners=[],calls=[];
  const api={runtime:{onMessage:{addListener:f=>listeners.push(f)},sendMessage:async({request})=>{calls.push(request.op);if(request.op==='permissions.status')return {error:'unpaired'};if(request.op==='permissions.request')return {id:'pair'};return {status:'approved'};}}};
  vm.runInNewContext(readFileSync('extensions/dist/chrome/src/content.js','utf8'),{chrome:api,window:{addEventListener(){},postMessage(){}},location:{origin:'https://example.com'},crypto:webcrypto,Uint8Array,setTimeout,Date});
  const pairing=listeners[0]({type:'TIMEBRIDGE_CONNECT',name:'Example'});
  assert.strictEqual(listeners[0]({type:'TIMEBRIDGE_CONNECT',name:'Example'}),pairing);
  assert.equal((await pairing).status,'approved');assert.deepEqual(calls,['permissions.status','permissions.request','permissions.poll']);
});
test('both extension popup manifests reference files that ship',()=>{
  for(const browser of ['chrome','firefox']){
    const root=`extensions/dist/${browser}/`,manifest=JSON.parse(readFileSync(root+'manifest.json'));
    const html=readFileSync(root+manifest.action.default_popup,'utf8');
    for(const [,file] of html.matchAll(/(?:src|href)="([^"]+)"/g))assert.ok(existsSync(root+file),`${browser}: missing ${file}`);
  }
});
test('built extensions preserve reviewed identities and least-privilege boundaries',()=>{
  const identity=JSON.parse(readFileSync('extensions/identity.json','utf8'));
  const chrome=JSON.parse(readFileSync('extensions/dist/chrome/manifest.json','utf8'));
  const firefox=JSON.parse(readFileSync('extensions/dist/firefox/manifest.json','utf8'));
  assert.equal(chrome.key,identity.key);
  assert.equal(firefox.browser_specific_settings.gecko.id,identity.firefoxId);
  assert.equal(firefox.browser_specific_settings.gecko.strict_min_version,'140.0');
  assert.equal(firefox.browser_specific_settings.gecko_android,undefined);
  assert.deepEqual(firefox.browser_specific_settings.gecko.data_collection_permissions,{required:['none']});
  for(const manifest of [chrome,firefox]){
    assert.deepEqual([...manifest.permissions].sort(),['activeTab','nativeMessaging','scripting','storage']);
    assert.equal(manifest.incognito,'not_allowed');
    assert.equal(manifest.host_permissions,undefined);
    assert.equal(manifest.content_scripts,undefined);
    assert.equal(manifest.web_accessible_resources,undefined);
    assert.equal(manifest.content_security_policy.extension_pages,"script-src 'self'; object-src 'none'");
  }
});
