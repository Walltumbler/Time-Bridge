# Architecture

Timebridge contains a Tauri desktop shell, Rust model/store/scheduler, a local JSON bridge, browser extensions, and a portable SDK. The desktop frontend derives remaining time from persisted UTC timestamps; it does not own the scheduler.

`model.rs` validates requests and implements recurrence and timer state transitions. `store.rs` writes state and events in SQLite transactions. `main.rs` supplies windows, tray, notifications, and a one-second scheduler. SQLite uses WAL and full synchronization. Item JSON evolves using defaults and explicit migrations where required.

The v1 bridge binds to IPv4 loopback only at `127.0.0.1:47832`. A caller must supply an exact allowed Origin and receive explicit approval in Desktop. Credentials are stored as hashes by the desktop and used for origin-scoped item access and event acknowledgement. Direct browser credentials are in memory by default; the extension retains its credentials in browser extension storage. Native applications can save credentials in their OS keychain.

The native-messaging helper accepts only bounded JSON, validates origins and credentials, and forwards to the fixed loopback endpoint. It exposes no shell or filesystem API. The extension reads the active tab origin only when the user connects that tab; it does not scrape the page or run on all visited sites.

The SDK fixes its transport after detection and never automatically replays a mutation through a second transport after a timeout. Native clients use fetch with an explicit application Origin. The JavaScript package is safe to import in Node/SSR; browser singleton creation is lazy. Any language able to send local JSON HTTP requests can implement the documented protocol.

A small, read-only setup site is embedded at compile time and served at `/setup/`. It serves only an explicit set of public resources, not files from the workspace. The same site can be hosted statically with the compiled SDK. The app offers the setup guide on first launch and in Settings.

Optional icons are embedded PNG data URLs, limited to 16 KiB and 128 × 128 pixels, validated by decoding the image. SVG, animated PNG, remote URLs, and malformed images are rejected. Icons are stored in item JSON, preserved when snoozed, and rendered as images with escaped attributes.

Audio uses one worker to serialize playback and temporary preferences. Windows routing addresses the selected endpoint directly, leaving the system default unchanged. A volume snapshot spans overlapping alarms; other apps' prior mute states are retained until the last sounding alarm stops. Normal exit restores settings. Device loss and process termination are documented limitations.

Weekly schedules use Monday=0 through Sunday=6 and IANA timezones. DST gaps are skipped, folds fire once, and overdue recurring occurrences are coalesced. The OS clock is authoritative. Website event acknowledgement is distinct from desktop notification delivery acknowledgement.
