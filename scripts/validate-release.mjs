import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const json = (file) => JSON.parse(readFileSync(path.join(root, file), 'utf8'));
const packageJson = json('package.json');
const release = json('release.config.json');
const tauri = json('src-tauri/tauri.conf.json');
const identity = json('extensions/identity.json');
const chrome = json('extensions/chrome/manifest.json');
const firefox = json('extensions/firefox/manifest.json');
const problems = [];
const expect = (condition, message) => { if (!condition) problems.push(message); };

for (const [name, version] of [
  ['package.json', packageJson.version],
  ['release.config.json', release.version],
  ['tauri.conf.json', tauri.version],
  ['Chrome manifest', chrome.version],
  ['Firefox manifest', firefox.version],
]) expect(version === release.version, `${name} version ${version} does not match ${release.version}`);

const key = Buffer.from(identity.key, 'base64');
expect(key.length > 0 && key.toString('base64') === identity.key, 'Chrome public key is not canonical base64');
const chromeId = [...createHash('sha256').update(key).digest().subarray(0, 16)]
  .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15))).join('');
expect(chromeId === identity.chromeId, `Chrome identity mismatch: key produces ${chromeId}`);
expect(chrome.key === '__CHROME_PUBLIC_KEY__', 'Source Chrome manifest must retain its generated-key placeholder');
expect(firefox.browser_specific_settings?.gecko?.id === identity.firefoxId, 'Firefox manifest ID does not match extensions/identity.json');
expect(firefox.browser_specific_settings?.gecko?.strict_min_version === '140.0', 'Firefox Desktop must require version 140 or newer for built-in data-consent support');
expect(!firefox.browser_specific_settings?.gecko_android, 'Firefox Android compatibility must remain disabled because native messaging requires the desktop application');
expect(JSON.stringify(firefox.browser_specific_settings?.gecko?.data_collection_permissions) === JSON.stringify({ required: ['none'] }), 'Firefox must explicitly declare that it collects or transmits no data');

const exactPermissions = ['activeTab', 'nativeMessaging', 'scripting', 'storage'].sort().join(',');
for (const [name, manifest] of [['Chrome', chrome], ['Firefox', firefox]]) {
  expect(manifest.manifest_version === 3, `${name} must use Manifest V3`);
  expect([...manifest.permissions].sort().join(',') === exactPermissions, `${name} permissions changed; review least privilege explicitly`);
  expect(!manifest.host_permissions, `${name} must not request persistent host permissions`);
  expect(!manifest.content_scripts, `${name} must inject only after an explicit toolbar action`);
  expect(!manifest.web_accessible_resources, `${name} must not expose packaged files to websites`);
  expect(manifest.incognito === 'not_allowed', `${name} must not mix normal-profile credentials with incognito tabs`);
  expect(manifest.content_security_policy?.extension_pages === "script-src 'self'; object-src 'none'", `${name} extension CSP changed`);
}

const runtimeFiles = ['extensions/chrome/src/background.js', 'extensions/chrome/src/content.js', 'extensions/chrome/popup.js'];
for (const file of runtimeFiles) {
  const source = readFileSync(path.join(root, file), 'utf8');
  expect(!/\beval\s*\(|\bnew\s+Function\s*\(/.test(source), `${file} contains dynamic code execution`);
  expect(!/https?:\/\//.test(source), `${file} contains a remote URL; extension runtime must be self-contained`);
}

for (const file of ['extensions/identity.json', 'src-tauri/extension-identity.json']) {
  expect(existsSync(path.join(root, file)), `${file} is missing`);
}
const nativeIdentity = json('src-tauri/extension-identity.json');
expect(nativeIdentity.chromeId === identity.chromeId && nativeIdentity.firefoxId === identity.firefoxId, 'Desktop native-host allowlist does not match extension identities');

if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}
console.log(`Release validation passed for Timebridge ${release.version}; Chrome ${identity.chromeId}; Firefox ${identity.firefoxId}.`);
