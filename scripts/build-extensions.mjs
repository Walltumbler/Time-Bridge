import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const identity = JSON.parse(await readFile(path.join(root, 'extensions/identity.json'), 'utf8'));
const iconDir = path.join(root, 'src-tauri/icons');
const dist = path.join(root, 'extensions/dist');

for (const browser of ['chrome', 'firefox']) {
  const output = path.join(dist, browser);
  await rm(output, { recursive: true, force: true });
  await mkdir(path.join(output, 'src'), { recursive: true });
  await mkdir(path.join(output, 'icons'), { recursive: true });
  await cp(path.join(root, 'extensions/chrome/src'), path.join(output, 'src'), { recursive: true });
  for (const file of ['popup.html', 'popup.css', 'popup.js']) {
    await cp(path.join(root, `extensions/chrome/${file}`), path.join(output, file));
  }
  await cp(path.join(iconDir, '32x32.png'), path.join(output, 'icons/32.png'));
  await cp(path.join(iconDir, '128x128.png'), path.join(output, 'icons/128.png'));
  const manifest = JSON.parse(await readFile(path.join(root, `extensions/${browser}/manifest.json`), 'utf8'));
  if (browser === 'chrome') manifest.key = identity.key;
  await writeFile(path.join(output, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
}

const chromeStore = path.join(dist, 'chrome-store');
await rm(chromeStore, { recursive: true, force: true });
await cp(path.join(dist, 'chrome'), chromeStore, { recursive: true });
const chromeStoreManifest = JSON.parse(await readFile(path.join(chromeStore, 'manifest.json'), 'utf8'));
delete chromeStoreManifest.key;
await writeFile(path.join(chromeStore, 'manifest.json'), `${JSON.stringify(chromeStoreManifest, null, 2)}\n`);

console.log(`Built keyed Chrome development (${identity.chromeId}), keyless Chrome Web Store, and Firefox (${identity.firefoxId}) extensions in extensions/dist/.`);

const validation = await import('node:child_process').then(({ spawnSync }) => spawnSync(process.execPath, [path.join(root, 'scripts/validate-release.mjs')], { stdio: 'inherit', cwd: root }));
if (validation.status !== 0) throw new Error(`Extension trust validation failed (${validation.status}).`);

const sdk = path.join(root, 'extensions/sdk/dist');
await rm(sdk, { recursive: true, force: true });
await mkdir(sdk, { recursive: true });
const tsc = path.join(root, 'node_modules/typescript/bin/tsc');
const result = await import('node:child_process').then(({ spawnSync }) => spawnSync(process.execPath, [tsc, '--target', 'ES2022', '--module', 'ES2022', '--moduleResolution', 'bundler', '--lib', 'ES2022,DOM', '--strict', '--declaration', '--outDir', sdk, path.join(root, 'extensions/sdk/index.ts'), path.join(root, 'extensions/sdk/types.ts')], { stdio: 'inherit' }));
if (result.status !== 0) throw new Error(`Website SDK TypeScript build failed (${result.status}).`);
console.log('Built typed website SDK in extensions/sdk/dist/.');

await cp(path.join(root, 'LICENSE'), path.join(root, 'extensions/sdk/LICENSE'));
