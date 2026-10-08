# Release process

The public repository is [Walltumbler/Time-Bridge](https://github.com/Walltumbler/Time-Bridge). It contains only the Timebridge product, generic examples, and Apache-2.0 source. Root ignore rules and `npm run audit:public` restrict publication candidates. Review the exact staged diff before publishing; never stage the entire working directory with force.

## Build artifacts

Use a clean Windows checkout with Node 22+, Rust stable, C++ build tools, and Windows SDK:

```sh
npm ci
npm run release:validate
npm run extensions:build
npm run test:sdk
npm run native:host
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo test --locked --manifest-path native-host/Cargo.toml
npm run desktop:build
npm run release:package
```

The preferred no-certificate trust path for Windows is an MSIX submission through Microsoft Store, which signs accepted Store packages. See [Microsoft Store packaging](microsoft-store.md). The direct-download MSI/EXE workflow remains prepared for trusted Authenticode signing; an unsigned local build cannot pass that signed-release workflow. If early testing requires a public unsigned direct-download build, publish it only under a preview tag, mark the GitHub release as a pre-release, label the Windows assets and release notes as unsigned, and reserve the final version tag for a signed workflow.

`artifacts/` contains the NSIS installer (and MSI when built), Chrome/Firefox store-submission ZIPs, SDK tarball, static website ZIP, `RELEASE-MANIFEST.json`, and `SHA256SUMS.txt`. The Chrome store ZIP is built from `extensions/dist/chrome-store` and intentionally omits the manifest `key`, which the Chrome Web Store rejects; `extensions/dist/chrome` remains a keyed unpacked development build. The manifest binds asset hashes to the product version and source revision; GitHub additionally attests the uploaded files. The ZIP roots contain their browser manifest directly. The native helper is bundled beside the executable and registered when Desktop opens. The setup site and SDK modules are compiled into Desktop; no workspace paths are served.

The **Prepare Windows release** Actions workflow builds and tests on a clean runner, uploads artifacts, and creates a **draft** GitHub release. Inspect and test the draft before publishing it. It refuses to replace a published release. Update versions consistently in package manifests, Tauri config, release config, extension manifests, SDK metadata, and release notes before a new version.

GitHub Actions are pinned to exact revisions, with Dependabot maintaining deliberate update pull requests for Actions, npm, and both Rust workspaces. Review those updates before merging; a green dependency update is not a substitute for reviewing permission or build-chain changes.

## User-friendly extension distribution

1. Create/complete publisher accounts for Chrome Web Store and Mozilla Add-ons.
2. Upload the browser ZIPs, provide descriptions/screenshots/privacy disclosures, and complete store review. Firefox needs signing even for permanent self-distribution.
3. Upload the keyless Chrome store ZIP, then record the store-assigned item ID. It must be added to the desktop native-host allowed origins before publishing the Chrome listing or the signed Desktop release. Keep the existing keyed ID in `extensions/identity.json` for unpacked development builds. Never commit private signing keys.
4. Confirm the Firefox ID matches `timebridge@timebridge.local` in the manifest and native-host allowlist. Complete any current Firefox data-collection declarations during submission; this project has no off-device collection.
5. Put the verified store URLs into `release.config.json`, rebuild Desktop and the static site, then publish the signed/listed add-ons. The setup guide switches from “pending” to Add to Chrome/Add to Firefox links.

Use [store-listing.md](store-listing.md) and [extension-privacy.md](extension-privacy.md) as the canonical store declarations. `npm run release:validate` rejects identity drift, added host permissions, automatic content scripts, exposed web resources, remote runtime URLs, dynamic code execution, and inconsistent versions. Chrome consumer packages become installable and signed only through the Chrome Web Store. Firefox packages become installable in release Firefox only after Mozilla signs them through AMO; a locally produced ZIP is a review input, not a signed consumer add-on.

Do not describe unpacked ZIPs as one-click consumer installs. Desktop cannot silently add the extension. The user approves browser installation and then separately approves each originating application in Timebridge.

## Desktop signing and site hosting

The checked-in direct-download workflow remains ready for a trusted Authenticode provider. A successful signed direct-download release must report `Valid` for the desktop executable, native helper, NSIS installer, and MSI. The Microsoft Store path instead uploads the unsigned MSIX described in [microsoft-store.md](microsoft-store.md); Microsoft signs it after acceptance. Signing improves publisher identity and tamper detection, but a new direct-download certificate may still need time to build Microsoft SmartScreen reputation.

`npm run site:build` produces `artifacts/site`. Host those static files together over HTTPS on GitHub Pages or another static host; all paths are relative so a project subpath works. Until hosting is enabled, the same guide works locally inside Desktop. The repository does not claim that a public site has already been deployed.

## Acceptance test

Install Desktop → open welcome guide → approve demo → create 10-second alarm with icon → close browser → verify visual/sound alert → snooze/dismiss → verify audio restoration. Repeat with the extension transport forced in Chrome and Firefox. Check denial, revocation, disconnected alarm output, and overlapping alarms. SDK native examples should create an alarm with Desktop open and a user approval. Record actual test results; do not substitute a passing build for OS/browser interaction tests.
