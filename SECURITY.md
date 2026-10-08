# Security

Timebridge is an early open-source desktop release. It exposes a local HTTP bridge on `127.0.0.1:47832`, a read-only bundled setup page, and a restricted browser native-messaging adapter. It has no cloud service or telemetry.

Applications require explicit approval in Desktop and origin-bound credentials. Do not place credentials in URLs, public source code, screenshots, or issue reports. Keep native-client credentials in secure OS storage. The desktop database is local and unencrypted.

Report vulnerabilities privately through this repository's private vulnerability reporting feature when available. If it is not configured, contact the repository maintainer privately before posting exploit details. A public release does not imply an independent security audit. Reports should include version, platform, reproduction, and impact, without personal timers or credentials.

See [the threat model](docs/security.md).
