# Browser extension privacy

The Timebridge browser extension has one purpose: after the user selects its toolbar button, it connects that website tab to the Timebridge desktop app on the same computer.

## Data handled

- The active tab's exact origin, such as `https://example.com`, is sent to the local Timebridge native-messaging helper when the user connects that tab.
- The website-provided app name, timer requests, and origin-scoped Timebridge credential are sent only between the extension and the local desktop helper.
- Origin-scoped credentials are stored in the browser extension's local storage so approved sites can reconnect. Website JavaScript cannot read that extension storage.

Timebridge does not build or retain a browsing-history list, scrape page content, read form contents, or collect analytics, advertising identifiers, or telemetry. It processes only the selected tab's origin and the Timebridge request deliberately supplied by an integrating website. It does not send extension data to the project maintainers or any remote Timebridge service. The extension does not run on every page: its content script is injected into the selected top-level tab only after the user selects the toolbar action.

## Chrome Web Store data-disclosure categories

The Chrome Web Store requires extensions to disclose data they process or store
even when it remains entirely on the user's device. Timebridge therefore selects
the following three standardized categories:

- **Web history:** Timebridge reads the origin of the single active website the
  user explicitly chooses to connect, for example `https://example.com`. The
  origin identifies which site is requesting permission and keeps its timers and
  credential separate from other sites. Timebridge does not read, build, or
  retain a list of websites the user visits.
- **Authentication information:** Timebridge Desktop generates a random pairing
  credential after the user approves a website. The extension stores that
  credential locally, scoped to the approved origin, and sends it only to the
  Timebridge native helper on the same computer. It is not a browser password or
  a Timebridge account password; Timebridge has no online accounts.
- **Website content:** An integrating website deliberately sends Timebridge the
  app name and timer, alarm, recurring-alarm, or stopwatch request needed for the
  requested feature. A request may also contain a small app icon or safe return
  URL. The extension does not scrape the page, read forms, or collect unrelated
  text, images, audio, video, or links.

These three categories are used only to connect an approved website to
Timebridge Desktop and run the timers or alarms requested by that website. The
data is not sold, shared with third parties, used for advertising or profiling,
or transmitted to developer-controlled servers. There is no cloud Timebridge
service.

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
