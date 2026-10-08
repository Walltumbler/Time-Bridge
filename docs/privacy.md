# Timebridge privacy policy

**Effective date:** October 8, 2026

**Publisher:** Yehoshua Shore

This policy applies to the Timebridge Windows desktop application, its native-messaging helper, and the official Timebridge browser extensions.

## Summary

Timebridge is a local desktop timer service. It does not require an account and the publisher does not operate a Timebridge cloud service. Timebridge does not collect, sell, rent, share, or transmit personal data, timer data, browsing data, analytics, advertising identifiers, or telemetry to the publisher or to third parties.

## Information processed on your device

Timebridge processes and stores the information required to provide timers and alarms:

- Timer, alarm, recurring-alarm, ranged-timer, and stopwatch details, including titles, schedules, status, history, and optional small app icons.
- Application settings, such as sound, notification, volume, and audio-output preferences.
- For a website you explicitly approve: its exact origin, display name, timer requests, permission status, and a cryptographic hash of its origin-scoped pairing credential.
- The browser extension stores the corresponding origin-scoped pairing credential in browser-local extension storage.
- A timer request may contain an optional safe return URL belonging to the same approved origin.

This information remains on your computer. The desktop application stores its data in its local application-data directory using SQLite. The browser extension stores its credentials in storage managed by the browser. Credentials separate one approved website from another and are not Timebridge account passwords or browser passwords.

## Browser access

The browser extension runs for a tab only after you select the Timebridge toolbar action. It reads the selected tab's origin and processes the Timebridge request deliberately provided by an integrating website. It does not scrape pages, read form contents, build a browsing-history list, monitor activity across websites, or run automatically on every page.

The extension and native helper communicate only with the Timebridge application on the same computer. The local HTTP bridge listens on the loopback interface (`127.0.0.1`) and is not a remote Timebridge service.

## Windows features

When enabled, Timebridge uses Windows notifications to display alarm information on your device. Optional audio controls may read available audio-device names and temporarily adjust the selected local output or volume while an alarm is sounding. These operations are local and are not reported to the publisher.

## Retention and your controls

Timer and event information remains on your device until you clear history, delete the relevant item, remove application data, or uninstall Timebridge, subject to Windows and browser data-retention behavior. You can revoke an approved website in Timebridge Desktop; revocation immediately makes its stored credential unusable. Removing the browser extension clears data controlled by that browser profile.

Because Timebridge does not receive or maintain a remote copy of this information, the publisher cannot retrieve, export, correct, or delete it on your behalf.

## Third-party services

Timebridge does not embed advertising, analytics, or third-party tracking services. Microsoft Store and your browser vendor may independently process download, purchase, installation, security, or diagnostic information under their own privacy policies. That processing is controlled by those services and is not data collected by Timebridge.

## Children

Timebridge is a general-purpose timer utility and is not directed to children. The application does not knowingly collect personal information from children or from any other users.

## Security

Timebridge uses explicit per-origin approval and origin-scoped credentials to restrict website access. Its local database is not encrypted, so anyone with access to your Windows account or application-data files may be able to read locally stored timer information. Do not put sensitive personal information in timer titles.

Security issues should be reported through the repository's [private vulnerability reporting process](../SECURITY.md) when available. Do not include personal timers or credentials in a public issue.

## Changes to this policy

Material changes will be published in this document with a revised effective date. The version in the public source repository is the current Timebridge privacy policy.

## Contact

For privacy questions, contact the publisher through the [Timebridge GitHub repository](https://github.com/Walltumbler/Time-Bridge/issues). Because issue reports are public, do not include credentials, private timer content, or other sensitive information.
