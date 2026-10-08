import {execFileSync} from 'node:child_process';
import {readFile,writeFile,mkdir,copyFile,readdir} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const config=JSON.parse(await readFile(path.join(root,'release.config.json'),'utf8'));
const artifacts=path.join(root,'artifacts');await mkdir(artifacts,{recursive:true});
execFileSync(process.execPath,[path.join(root,'scripts/audit-public.mjs')],{stdio:'inherit',cwd:root});
execFileSync(process.execPath,[path.join(root,'scripts/build-extensions.mjs')],{stdio:'inherit',cwd:root});
execFileSync(process.execPath,[path.join(root,'scripts/build-site.mjs')],{stdio:'inherit',cwd:root});
const npm=process.env.npm_execpath || path.join(root,'.tools/package/bin/npm-cli.js');
if(!existsSync(npm))throw new Error('Run this through npm run release:package.');
execFileSync(process.execPath,[npm,'pack',path.join(root,'extensions/sdk'),'--pack-destination',artifacts],{stdio:'inherit',cwd:root});
const outputs=[`timebridge-sdk-${config.version}.tgz`];
const quote=value=>`'${value.replaceAll("'","''")}'`;
for(const [folder,name] of [['extensions/dist/chrome',`Timebridge-Chrome-${config.version}.zip`],['extensions/dist/firefox',`Timebridge-Firefox-${config.version}.zip`],['artifacts/site',`Timebridge-Website-${config.version}.zip`]]){
  const source=path.join(root,folder),output=path.join(artifacts,name);
  if(process.platform==='win32')execFileSync('powershell.exe',['-NoProfile','-NonInteractive','-Command',`$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; if (Test-Path -LiteralPath ${quote(output)}) { Remove-Item -LiteralPath ${quote(output)} }; $sourceRoot=${quote(source)}; $archive=[System.IO.Compression.ZipFile]::Open(${quote(output)}, 'Create'); try { Get-ChildItem -LiteralPath $sourceRoot -Recurse -File | ForEach-Object { $entryName=$_.FullName.Substring($sourceRoot.Length+1).Replace([char]92,[char]47); [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive,$_.FullName,$entryName) | Out-Null } } finally { $archive.Dispose() }`],{stdio:'inherit'});
  else execFileSync('python3',['-c','import pathlib,sys,zipfile; root=pathlib.Path(sys.argv[1]); z=zipfile.ZipFile(sys.argv[2],"w",zipfile.ZIP_DEFLATED); [z.write(p,p.relative_to(root)) for p in root.rglob("*") if p.is_file()]; z.close()',source,output],{stdio:'inherit'});
  outputs.push(name);
}
let installers=0;
for(const kind of ['nsis','msi']){
  const directory=path.join(root,'src-tauri/target/release/bundle',kind);if(!existsSync(directory))continue;
  for(const name of await readdir(directory))if(name.includes(config.version)&&/\.(exe|msi)$/.test(name)){await copyFile(path.join(directory,name),path.join(artifacts,name));outputs.push(name);installers++;}
}
if(!installers)throw new Error('Build the desktop release before packaging installers.');
const sums=[];for(const name of outputs)sums.push(`${createHash('sha256').update(await readFile(path.join(artifacts,name))).digest('hex')}  ${name}`);
await writeFile(path.join(artifacts,'SHA256SUMS.txt'),sums.join('\n')+'\n');
console.log(`Prepared ${outputs.length} release assets and SHA256SUMS.txt. Nothing was published.`);
