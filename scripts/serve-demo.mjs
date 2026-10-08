import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const files=new Map([
  ['/', ['site/index.html','text/html']], ['/style.css',['site/style.css','text/css']],
  ['/app.js',['site/app.js','text/javascript']], ['/release.config.json',['release.config.json','application/json']],
  ...['index','client','types','icons'].map(name=>[`/sdk/${name}.js`,[`extensions/sdk/dist/${name}.js`,'text/javascript']]),
]);
createServer(async(req,res)=>{
  const file=files.get(new URL(req.url,'http://127.0.0.1:4790').pathname);
  if(req.headers.host!=='127.0.0.1:4790'||req.method!=='GET'||!file){res.writeHead(404);res.end('Not found');return;}
  try{const body=await readFile(path.join(root,file[0]));res.writeHead(200,{'Content-Type':`${file[1]}; charset=utf-8`,'Cache-Control':'no-store','X-Content-Type-Options':'nosniff'});res.end(body);}
  catch{res.writeHead(503);res.end('Build the SDK first: npm run extensions:build');}
}).listen(4790,'127.0.0.1',()=>console.log('Timebridge setup and SDK demo: http://127.0.0.1:4790'));
