# Contributing

Timebridge is a standalone, application-neutral project. Examples should use generic names and `example.com` domains. Do not submit unrelated application source, private handoff documents, credentials, timer databases, machine-specific paths, or generated build trees.

Use Node 22+, Rust stable, and the Tauri platform prerequisites. Follow the README build order so the compiled SDK and native helper exist before the desktop build. Add focused tests for protocol, persistence, permissions, or scheduling changes. Run `npm run test:sdk` and the Rust library/helper tests. Interactive audio tests are ignored by default because they change real device settings and play a brief sound.

Run `npm run audit:public` before committing. This checks the explicitly permitted project files for common accidental data/credential leaks; it does not replace review. Build output is distributed as release assets, not source commits.

Contributions use the repository's Apache-2.0 license. Store listings, signing credentials, and publisher credentials must never be committed.
