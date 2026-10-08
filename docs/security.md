# Threat model

## Protected data

Timer titles, schedules, icons, app origins, return URLs, preferences, and event history remain in the application data directory. No telemetry, cloud synchronization, or account system is present. SQLite is not encrypted; OS account access and disk encryption define its at-rest boundary.

## Boundaries

- Only local packaged Tauri windows receive desktop commands. Untrusted sites use the separate bridge.
- The HTTP bridge binds to `127.0.0.1:47832`, requires an exact Host and a valid exact HTTPS Origin (HTTP only for localhost development), and accepts bounded, strict JSON schemas.
- Pairing needs a desktop user decision, an expiring poll secret, and a credential scoped to the exact origin. Desktop stores only credential hashes. Revocation invalidates access and pending credential claims.
- Item reads, updates, events, and acknowledgement are origin-scoped. Source identity is assigned by Desktop; callers cannot supply source metadata. Quotas, rate limits, concurrency limits, and body limits constrain abuse.
- Browser Origin is browser-controlled. Native app origins are self-declared, so the approval name/origin is a user-facing identifier, not proof that the process owns a domain. Credentials remain necessary.
- Return URLs must match the paired origin, have no embedded credentials, and use HTTPS or exact localhost development HTTP.
- The frontend escapes user text. Icon images are bounded, fully decoded static PNGs; no arbitrary remote images or SVG are accepted. The setup server serves an explicit compiled asset list and does not expose workspace files.
- The SDK never retries a timed-out write over a second transport. Callers must check results before retrying operations.
- The native helper has no generic shell/file interface. Browser credentials stay in extension storage; page requests do not receive them.
- Audio overrides are desktop user preferences, unavailable to website callers.

## Limits

An attacker already running as the OS user can read local files and impersonate local HTTP clients. CORS is not authentication. This app does not defend against OS-account compromise. The public setup site can request local-network permission; users may choose the browser extension instead. Extension and desktop signing/store review, dependency auditing, fuzzing, and independent review remain release work.
