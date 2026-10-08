## Timebridge 0.2.0 preview 1

This is the first public Timebridge preview: persistent local timers and alarms, Windows visual/audio alerts, browser and Node SDK, Chrome/Firefox development extensions, and a bundled setup/demo page.

> **Unsigned preview:** The Windows installers in this pre-release are not code-signed. Windows may show an unknown-publisher warning. A SignPath Foundation-signed stable release is pending. Do not treat this preview as the final trusted distribution.

- Download the Windows setup executable, install, and open Timebridge.
- Use the welcome guide to connect a page and create a 10-second test alarm.
- Applications can attach a small PNG icon to distinguish alarms.
- The extension ZIPs are development builds: Chrome uses Load unpacked; Firefox uses temporary installation. Store listings and Firefox signing are still pending.
- SDK: install the included `timebridge-sdk-0.2.0.tgz`; no npm publication is implied.
- Windows binaries are unsigned. The GitHub release must remain marked as a pre-release. macOS release validation is pending.
- Compare downloaded assets against SHA256SUMS.txt.

Desktop must be running and the computer awake. Audio overrides are opt-in and restore after the final sounding alarm stops or normal application exit.
