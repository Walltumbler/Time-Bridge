import {execFileSync} from 'node:child_process';
for(const args of [['scripts/build-extensions.mjs'],['node_modules/typescript/bin/tsc','--noEmit'],['node_modules/vite/bin/vite.js','build']])execFileSync(process.execPath,args,{stdio:'inherit'});
