import test from 'node:test';
import assert from 'node:assert/strict';
import {createTimebridge,TimebridgeError} from '../extensions/sdk/dist/index.js';

test('Node imports safely, sends its origin, pairs, and creates an icon alarm without inventing a return URL',async()=>{
  const calls=[];let saved;
  const client=createTimebridge({origin:'https://example.com',appName:'Example',onCredential:value=>{saved=value;},fetch:async(url,options)=>{
    assert.equal(url,'http://127.0.0.1:47832/v1');assert.equal(options.headers.Origin,'https://example.com');
    const body=JSON.parse(options.body);calls.push(body);
    if(body.op==='detect')return Response.json({desktop:true,version:'0.2.0'});
    if(body.op==='permissions.status')return Response.json({error:'Site is not paired'},{status:401});
    if(body.op==='permissions.request'){assert.match(body.pollSecret,/^[a-f0-9]{64}$/);return Response.json({id:'pair'});}
    if(body.op==='permissions.poll')return Response.json({status:'approved',credential:'a'.repeat(64)});
    assert.equal(options.headers.Authorization,`Bearer ${'a'.repeat(64)}`);
    return Response.json({id:'alarm',...body.input});
  }});
  await client.requestPermission();assert.equal(saved,'a'.repeat(64));
  const alarm=await client.createCountdown({durationSeconds:10,icon:'data:image/png;base64,example'});
  assert.equal(alarm.icon,'data:image/png;base64,example');assert.equal('returnUrl' in calls.at(-1).input,false);
  await client.createWeeklyAlarm({weekdays:[0],time:'09:00',timezone:'UTC'});
  assert.deepEqual(calls.at(-1).input,{kind:'recurring_alarm',schedule:{kind:'weekly',weekdays:[0],time:'09:00',timezone:'UTC'}});
  client.dispose();assert.throws(()=>client.requestPermission(),e=>e.code==='CLIENT_DISPOSED');
});
test('paired native client reuses credentials and rejects cross-origin returns',async()=>{
  const calls=[];
  const client=createTimebridge({origin:'https://example.com',credential:'a'.repeat(64),fetch:async(_,o)=>{const q=JSON.parse(o.body);calls.push(q.op);return Response.json(q.op==='detect'?{desktop:true}:{approved:true});}});
  await client.requestPermission();assert.deepEqual(calls,['detect','permissions.status']);
  assert.throws(()=>client.createAlarm({fireAt:new Date().toISOString(),timezone:'UTC',returnUrl:'https://other.example/task'}),e=>e.code==='INVALID_RETURN_URL');client.dispose();
});
test('browser errors reject and a timed-out write is never retried through loopback',async()=>{
  const listeners=new Set();let fetches=0;
  globalThis.window={location:new URL('https://example.com/demo'),addEventListener:(_,f)=>listeners.add(f),removeEventListener:(_,f)=>listeners.delete(f),postMessage(message){
    let result;
    if(message.request.op==='detect')result={desktop:true};
    else if(message.request.op==='items.list')result={error:'Site is not paired'};
    else return;
    queueMicrotask(()=>{for(const f of listeners)f({source:window,origin:'https://example.com',data:{source:'timebridge-extension',id:message.id,result}});});
  }};
  globalThis.document={title:'Example',querySelector:()=>null};
  const client=createTimebridge({timeoutMs:20,fetch:async()=>{fetches++;return Response.json({desktop:true});}});
  try {assert.equal((await client.connect()).transport,'extension');await assert.rejects(client.listItems(),e=>e instanceof TimebridgeError&&e.code==='PAIRING_REQUIRED');await assert.rejects(client.createCountdown({durationSeconds:10}),e=>e.code==='EXTENSION_TIMEOUT');assert.equal(fetches,0);}
  finally{client.dispose();delete globalThis.window;delete globalThis.document;}
});
