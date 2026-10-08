import { execFileSync } from 'node:child_process';
import { mkdirSync, copyFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const debug = process.argv.includes('--debug'), profile = debug ? 'debug' : 'release';
execFileSync('cargo', ['build', '--locked', '--manifest-path', path.join(root,'native-host/Cargo.toml'), ...(debug ? [] : ['--release'])], {stdio:'inherit'});
const triple=execFileSync('rustc',['-vV'],{encoding:'utf8'}).match(/^host: (.+)$/m)?.[1];
if(!triple)throw new Error('Cannot determine Rust target');
const suffix=process.platform==='win32'?'.exe':'';
const binary=path.join(root,`native-host/target/${profile}/timebridge-native-host${suffix}`);
mkdirSync(path.join(root,'src-tauri/binaries'),{recursive:true});
copyFileSync(binary,path.join(root,`src-tauri/binaries/timebridge-host-${triple}${suffix}`));
// Unpackaged development builds need the same sidecar name as installed builds.
mkdirSync(path.join(root,`src-tauri/target/${profile}`),{recursive:true});
copyFileSync(binary,path.join(root,`src-tauri/target/${profile}/timebridge-host${suffix}`));
console.log(`Native helper prepared for ${triple}.`);
