import {execFileSync} from 'node:child_process';
import {readFileSync,lstatSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const roots=new Set(['.github','docs','examples','extensions','native-host','scripts','site','src','src-tauri','store','tests']);
const top=new Set(['.gitignore','LICENSE','README.md','SECURITY.md','CONTRIBUTING.md','package.json','package-lock.json','release.config.json','index.html','tsconfig.json','vite.config.ts','googlee913e7a9fe3d6977.html']);
const files=[...new Set(execFileSync('git',['ls-files','-z','--cached','--others','--exclude-standard'],{cwd:root,encoding:'utf8'}).split('\0').filter(Boolean))].sort();
const forbidden=/(^|\/)(node_modules|target|dist|artifacts|\.tools|\.git|gen|resources|binaries)(\/|$)|\.db(?:-|$)|\.(?:pem|key|pfx|p12|log|exe|msi|zip|tgz)$|(^|\/)\.env|handoff/i;
const problems=[];
for(const file of files){
  if(!top.has(file)&&!roots.has(file.split('/')[0])){problems.push(`${file}: outside public file list`);continue;}
  if(forbidden.test(file)){problems.push(`${file}: generated, local, or sensitive file`);continue;}
  const stat=lstatSync(path.join(root,file));if(stat.isSymbolicLink()){problems.push(`${file}: symbolic link not allowed`);continue;}
  if(stat.size>2_000_000){problems.push(`${file}: unexpectedly large source file`);continue;}
  if(/\.(png|ico|icns|wav)$/.test(file))continue;
  const content=readFileSync(path.join(root,file),'utf8');
  if(/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/.test(content)||/\b(?:ghp_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/.test(content))problems.push(`${file}: possible credential`);
  if(/[A-Z]:[\\/]+Users[\\/]+[^\s"'<>]+/i.test(content))problems.push(`${file}: personal absolute path`);
}
if(problems.length){console.error(problems.join('\n'));process.exitCode=1;}
else{console.log(`Public source audit passed: ${files.length} files. Generated output and unrelated workspace files are excluded.`);if(process.argv.includes('--list'))console.log(files.join('\n'));}
