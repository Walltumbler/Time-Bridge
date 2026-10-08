# Browser extension privacy

The Timebridge browser extension has one purpose: after the user selects its toolbar button, it connects that website tab to the Timebridge desktop app on the same computer.

## Data handled

- The active tab's exact origin, such as `https://example.com`, is sent to the local Timebridge native-messaging helper when the user connects that tab.
- The website-provided app name, timer requests, and origin-scoped Timebridge credential are sent only between the extension and the local desktop helper.
- Origin-scoped credentials are stored in the browser extension's local storage so approved sites can reconnect. Website JavaScript cannot read that extension storage.

Timebridge does not collect browsing history, page content, form contents, analytics, advertising identifiers, or telemetry. It does not send extension data to the project maintainers or any remote Timebridge service. The extension does not run on every page: its content script is injected into the selected top-level tab only after the user selects the toolbar action.

The Firefox package declares `browser_specific_settings.gecko.data_collection_permissions.required` as `["none"]`, which exposes this no-data-collection commitment directly in Firefox's installation and add-on permission interface.

## Permissions

- `activeTab`: reads the selected tab's origin after a user gesture.
- `scripting`: injects the small Timebridge page bridge into that selected tab.
- `nativeMessaging`: communicates with the locally installed Timebridge desktop helper.
- `storage`: retains a separate credential for each user-approved origin.

The extension requests no persistent host permissions and exposes no web-accessible resources. Incognito operation is disabled to avoid mixing normal-profile approvals with private browsing.

## Control and retention

Users can revoke a website in Timebridge Desktop. Revocation immediately makes its stored credential unusable. Removing the extension clears data controlled by the browser. Removing Timebridge Desktop does not transmit or upload extension data.

Security reports should follow [SECURITY.md](../SECURITY.md).
