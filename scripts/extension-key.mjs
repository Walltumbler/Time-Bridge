import {generateKeyPairSync,createHash} from 'node:crypto';
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs';
mkdirSync('extensions',{recursive:true});
const file='extensions/identity.json';
if(!existsSync(file)){
  const {publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
  const der=publicKey.export({type:'spki',format:'der'});
  const chromeId=[...createHash('sha256').update(der).digest().subarray(0,16)].map(b=>String.fromCharCode(97+(b>>4),97+(b&15))).join('');
  writeFileSync(file,JSON.stringify({key:der.toString('base64'),chromeId,firefoxId:'timebridge@timebridge.local'},null,2)+'\n');
}
const {chromeId,firefoxId}=JSON.parse(readFileSync(file));
writeFileSync('src-tauri/extension-identity.json',JSON.stringify({chromeId,firefoxId},null,2)+'\n');
