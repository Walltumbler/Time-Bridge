# Development verification

## SDK, app icons, and installation — 2026-10-08

- All 25 regular Rust library tests pass; two desktop audio tests remain opt-in. The HTTP integration test covers pairing approval, credentials, creating an icon-bearing alarm, and reading it back with origin isolation.
- Six JavaScript tests pass, including SDK transport errors, credential reuse, browser extension pairing, and preventing duplicate writes after a timeout. TypeScript/Vite production build, Rust all-target checks, and the native helper test pass.
- Windows release executable, NSIS installer, and MSI build successfully. Release packaging produces the SDK tarball, Chrome/Firefox development archives, static setup website, both installers, and SHA-256 checksums.
- The running desktop serves the bundled setup page and SDK. Store publication, installer signing, a public hosted website, and npm publication are not complete. Installation and a live browser alarm still need end-user testing.
- Public-source allowlist and staged-content checks exclude unrelated workspace projects, generated files, credentials, and local user data.

## Alarm audio preferences — 2026-10-07

- Added opt-in Windows volume override (default target 75%), persistent alarm output-device selection, and temporary muting of other apps.
- 21 automated Rust tests pass, including legacy-settings defaults, validation/persistence, overlapping volume leases, rollback after partial failure, and bundled waveform parsing. TypeScript production build and all-target Rust checks pass.
- Two explicit tests on the interactive Windows desktop pass: setting the chosen endpoint to 75%, playing a short alarm, and restoring endpoint/app volume and mute state; muting and restoring eight other audio sessions. The Windows default endpoint remains unchanged.
- A single audio worker serializes playback, overrides, and cleanup. It restores after the final sounding alarm stops and on normal application exit. Tests do not establish what a user actually hears, nor cover physical unplug/replug, forced termination, or macOS/Linux audio overrides.
- Device routing uses Windows endpoint IDs mapped to waveform devices, per Microsoft's [endpoint ID documentation](https://learn.microsoft.com/en-us/windows/desktop/CoreAudio/endpoint-id-strings).

## Original implementation checks (before subsequent updates)

Verified on Windows on 2026-10-07:

- TypeScript strict type checking and Vite production build passed.
- Rust desktop executable compiled successfully with Tauri 2.
- 13 Rust tests passed: countdown pause/resume, ranged minimum/maximum transitions, overdue recovery, SQLite reopen/restart, cancellation persistence, source URL restrictions, invalid input, IANA timezone recurrence, DST gap/fold handling, date-list catch-up, stopwatch pauses, durable notification backlog, and snooze preserving recurrence.
- NSIS development installer generated: `src-tauri/target/debug/bundle/nsis/Timebridge_0.1.0_x64-setup.exe`.
- Native application launched and created its SQLite database. Process was responsive.
- Browser design preview visually inspected; creation form and weekly scheduling controls inspected.

Not verified in this environment:

- Native UI interaction, visible native notification delivery, audible output, and tray interactions. Computer-use app-access approval timed out before native window capture.
- macOS build/runtime behavior. CI configuration includes macOS, but it has not been run here.
- Installer execution and OS notification identity after installation. The installer is unsigned.

Before calling this a release, run a short timer with the window closed, test range milestones, pause/resume, snooze/dismiss, restart while overdue, and repeat notification/tray checks on macOS. The automated tests exercise scheduler restart semantics; they do not replace OS-level sleep/restart validation.
