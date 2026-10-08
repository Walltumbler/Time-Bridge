# Release process

The public repository is [Walltumbler/Time-Bridge](https://github.com/Walltumbler/Time-Bridge). It contains only the Timebridge product, generic examples, and Apache-2.0 source. Root ignore rules and `npm run audit:public` restrict publication candidates. Review the exact staged diff before publishing; never stage the entire working directory with force.

## Build artifacts

Use a clean Windows checkout with Node 22+, Rust stable, C++ build tools, and Windows SDK:

```sh
npm ci
npm run extensions:build
npm run test:sdk
npm run native:host
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo test --locked --manifest-path native-host/Cargo.toml
npm run desktop:build
npm run release:package
```

`artifacts/` contains the NSIS installer (and MSI when built), Chrome/Firefox ZIPs, SDK tarball, static website ZIP, and SHA256SUMS.txt. The ZIP roots contain their manifest directly. The native helper is bundled beside the executable and registered when Desktop opens. The setup site and SDK modules are compiled into Desktop; no workspace paths are served.

The **Prepare Windows release** Actions workflow builds and tests on a clean runner, uploads artifacts, and creates a **draft** GitHub release. Inspect and test the draft before publishing it. It refuses to replace a published release. Update versions consistently in package manifests, Tauri config, release config, extension manifests, SDK metadata, and release notes before a new version.

## User-friendly extension distribution

1. Create/complete publisher accounts for Chrome Web Store and Mozilla Add-ons.
2. Upload the browser ZIPs, provide descriptions/screenshots/privacy disclosures, and complete store review. Firefox needs signing even for permanent self-distribution.
3. Verify final extension identities. Chrome's store-assigned identity must match the public key/ID in `extensions/identity.json`, `extensions/chrome/manifest.json`, and the desktop native-host allowed origin in `src-tauri/extension-identity.json`. If the store assigns a different key, regenerate the public identity files with the store-provided key and rebuild Desktop. Never commit private signing keys.
4. Confirm the Firefox ID matches `timebridge@timebridge.local` in the manifest and native-host allowlist. Complete any current Firefox data-collection declarations during submission; this project has no off-device collection.
5. Put the verified store URLs into `release.config.json`, rebuild Desktop and the static site, then publish the signed/listed add-ons. The setup guide switches from “pending” to Add to Chrome/Add to Firefox links.

Do not describe unpacked ZIPs as one-click consumer installs. Desktop cannot silently add the extension. The user approves browser installation and then separately approves each originating application in Timebridge.

## Desktop signing and site hosting

The current Windows installer is unsigned. Configure a signing certificate or a supported signing service using CI secrets when ready; do not commit certificate files or passwords. Validate a clean installation, browser helper registration, upgrade, and uninstall on a test machine before announcing a stable release.

`npm run site:build` produces `artifacts/site`. Host those static files together over HTTPS on GitHub Pages or another static host; all paths are relative so a project subpath works. Until hosting is enabled, the same guide works locally inside Desktop. The repository does not claim that a public site has already been deployed.

## Acceptance test

Install Desktop → open welcome guide → approve demo → create 10-second alarm with icon → close browser → verify visual/sound alert → snooze/dismiss → verify audio restoration. Repeat with the extension transport forced in Chrome and Firefox. Check denial, revocation, disconnected alarm output, and overlapping alarms. SDK native examples should create an alarm with Desktop open and a user approval. Record actual test results; do not substitute a passing build for OS/browser interaction tests.
