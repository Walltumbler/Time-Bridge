# Browser store listing

Use this reviewed copy for the Chrome Web Store and Mozilla Add-ons listings.

## Summary

Hand website timers to the local Timebridge desktop app so they keep running after the tab closes.

## Single purpose

Timebridge connects a website selected by the user to the Timebridge desktop timer service installed on the same computer. It does not provide general browser automation or inspect page content.

## Permission justifications

| Permission | Justification |
| --- | --- |
| `activeTab` | Identify and connect only the tab on which the user selected the extension. |
| `scripting` | Inject the packaged Timebridge bridge into that selected tab after the toolbar action. |
| `nativeMessaging` | Send bounded timer protocol messages to the installed local desktop helper. |
| `storage` | Retain origin-scoped approval credentials inside extension storage. |

## Data-use declarations

- No sale or transfer of user data.
- No advertising, analytics, tracking, or profiling.
- No remote code or remotely hosted executable logic.
- No browsing-history or page-content collection.
- Local processing only; see [extension-privacy.md](extension-privacy.md).

Before submission, replace pending URLs in `release.config.json`, verify the store-assigned Chrome public key and item ID, and use the screenshots produced from the exact submitted version.
