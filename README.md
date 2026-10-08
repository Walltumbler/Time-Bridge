# Timebridge

An open-source desktop home for timers and alarms from any app. Create a countdown in a website, close the page, and let Timebridge keep the time.

**Windows desktop app · Apache-2.0 · No cloud account**

## Get started

1. Download the Windows `Timebridge_*_x64-setup.exe` from [Releases](https://github.com/Walltumbler/Time-Bridge/releases). Install it and open Timebridge.
2. Choose **Connect an app or website** on the welcome screen, or **Settings → Open setup guide & test alarm**.
3. Connect the demo, approve its origin in Desktop, and create a 10-second alarm. A browser extension is optional when a direct local connection works.

The setup guide is bundled into the desktop app at `http://127.0.0.1:47832/setup/`. It includes troubleshooting and Chrome/Firefox extension instructions. If Releases is empty, this repository is still source-only; see development instructions below. Chrome and Firefox store listings are not published yet. [Installation guide](docs/install.md).

## Integrate your app

The JavaScript/TypeScript SDK supports browsers and Node 22+. Native apps in other languages can use the same local JSON API. Both endpoints must run on the same user's computer; a remote server cannot ring someone's PC directly.

```js
import { createTimebridge } from '@timebridge/sdk';

const clock = createTimebridge({ appName: 'Example app' });
await clock.requestPermission(); // User approves this origin in Desktop.
await clock.createCountdown({ durationSeconds: 60 });
```

Names are optional. Pass `icon: 'data:image/png;base64,...'` to distinguish your alarms. Small icons are saved locally and shown on timer cards and ringing alerts, including after restart or snooze.

The SDK package is currently distributed as a release tarball or built from this repository; it is not yet published to npm. [SDK guide and examples](docs/sdk.md) · [Protocol](docs/protocol.md).

## Features

- Countdowns, ranged timers, scheduled and recurring alarms, and stopwatches.
- SQLite persistence, overdue recovery, and independent desktop scheduling.
- Visual alarm window, repeating sound, snooze, silence, and history.
- Optional Windows alarm output device, temporary volume override (75% initially), and muting other apps with restoration afterward.
- Explicit per-origin app approval, revocation, origin-scoped items and events.
- Browser extensions for Chrome and Firefox; a browser/Node SDK and Python example.
- Bundled setup guide and a real 10-second SDK demo with selectable icons.

## Development

Install Node 22+, Rust stable, and [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). On Windows install the C++ desktop build tools and Windows SDK. Then:

```sh
npm ci
npm run extensions:build
npm run native:host:debug
npm run build
npm run desktop
```

`npm run desktop` runs the Vite development server and the native app together. `npm run dev` alone is a design preview with sample items, not a persistent timer service. For the SDK setup page without rebuilding Desktop, run `npm run demo` and open `http://127.0.0.1:4790`.

```sh
npm run test:sdk
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo test --locked --manifest-path native-host/Cargo.toml
npm run desktop:build
npm run release:package
```

Stable release artifacts include SignPath Foundation-signed Windows executables/installers, extension store-submission ZIPs, an SDK tarball, the static setup/demo website, SHA-256 checksums, and GitHub build provenance. An explicitly marked GitHub pre-release may be made available for early testing before signing is approved; its Windows installers are not a trusted final distribution. Consumer browser extensions must be installed through their verified Chrome Web Store or Mozilla Add-ons listing. [Release process](docs/releasing.md) · [Extension privacy](docs/extension-privacy.md) · [Contributing](CONTRIBUTING.md).

## Limits

Desktop must be running and the computer awake. Closing the main window leaves the tray service running; quitting stops alerts. System notification policies may suppress notifications. Local development builds may be unsigned; the public release workflow requires valid Authenticode signatures. The current enhanced audio controls are Windows-only; macOS release behavior remains unverified. Audio restoration runs on normal alarm stop and normal exit; forced termination cannot perform cleanup.

The user controls volume and output preferences. Websites cannot select a device or override those preferences. Each approved app manages only its own items. Revoking access keeps existing timers under the user's control. Signing identifies the publisher and detects tampering; it does not replace store review or an independent security audit.

[Privacy](docs/privacy.md) · [Security](SECURITY.md) · [Architecture](docs/architecture.md) · [Verification](docs/verification.md) · [Apache-2.0 license](LICENSE)
