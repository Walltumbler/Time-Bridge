# Timebridge support

Timebridge is an open-source Windows desktop timer service published by Yehoshua Shore. The Microsoft Store product ID is `9PPL5ZBK8L76`.

## Get help

For installation problems, browser-extension connection problems, timer behavior, or other support questions, open a request in the public [Timebridge issue tracker](https://github.com/Walltumbler/Time-Bridge/issues/new/choose). Include the Timebridge version, Windows version, browser and extension version when relevant, steps to reproduce the problem, and any error message shown by the application.

Do not include pairing credentials, private timer titles, personal data, or other secrets in a public issue. Potential security vulnerabilities should be reported using the private process described in the [security policy](../SECURITY.md).

## Browser connection requirements

The Timebridge Windows application and browser add-on are installed separately. Install the desktop application from Microsoft Store and the official Timebridge add-on from the applicable browser store. Launch Timebridge once, choose the add-on's Connect action in the website tab, and approve that exact website origin in the desktop application.

The desktop service must remain running in the Windows system tray for timers and alarms to continue after the browser closes. Fully quitting Timebridge stops its timer service and alerts.

## Common checks

- Confirm Timebridge is running in the system tray.
- Confirm the website origin displayed by Timebridge is the site you intended to approve.
- Confirm the browser add-on reports that the desktop service is connected.
- Revoke and reconnect the website if its saved permission is no longer valid.
- Keep the computer awake; Windows cannot display an alarm while the device is powered off.

## Project information

- [Source code and releases](https://github.com/Walltumbler/Time-Bridge)
- [Installation guide](install.md)
- [Privacy policy](privacy.md)
- [Security policy](../SECURITY.md)
- [Apache License 2.0](../LICENSE)

Timebridge has no cloud account, advertising, analytics, or telemetry. Timer information and approved-website data remain on the user's computer.
