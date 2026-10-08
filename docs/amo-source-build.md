# Firefox extension source build

This document describes how to reproduce the Firefox extension submitted to
Mozilla Add-ons. The extension is plain JavaScript, HTML, CSS, JSON, and PNG. It
is not minified, obfuscated, bundled, or transpiled. The build script copies the
shared extension files, writes the Firefox manifest, copies the icons, and runs
release checks.

## Build environment

- Operating system: Ubuntu 24.04 or Windows 11
- Node.js: 24.x (tested with 24.19.0)
- npm: use the npm version supplied with Node.js; dependencies are locked by
  `package-lock.json`
- Network access is needed only for `npm ci`

Node.js and npm can be installed from https://nodejs.org/

No Rust toolchain, Tauri build, desktop application, signing certificate, or
environment variables are required to build the Firefox extension.

## Reproduce the submitted extension

From the repository root, run:

```sh
npm ci
npm run extensions:build
```

The complete unpacked Firefox extension will be written to:

```text
extensions/dist/firefox
```

Compare the files inside that directory with the files at the root of the
submitted Firefox ZIP. ZIP container metadata can vary by archiving tool; the
file names and file contents are the reproducible output.

The build also creates `extensions/dist/chrome` and `extensions/sdk/dist`.
Those directories are not part of the submitted Firefox extension.

## Source layout

- `extensions/firefox/manifest.json`: Firefox manifest source
- `extensions/chrome/src/`: shared extension JavaScript source
- `extensions/chrome/popup.*`: shared popup source
- `src-tauri/icons/`: source icons copied into the extension
- `scripts/build-extensions.mjs`: extension build script
- `scripts/validate-release.mjs`: identity and security validation
- `extensions/identity.json` and `src-tauri/extension-identity.json`: extension
  and native-host identities checked by the validator

The separately distributed Timebridge desktop service is not built or included
in the Firefox extension package.
