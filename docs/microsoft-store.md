# Microsoft Store packaging

The preferred trust path for Timebridge on Windows is an MSIX submission through Microsoft Partner Center. Microsoft signs an accepted Store package, so the project does not need to purchase an Authenticode certificate merely to distribute through the Store.

## Why this package requires two restricted capabilities

Timebridge is a desktop timer service with a browser-extension bridge. Chrome and Firefox discover that bridge through per-user native-messaging registration. MSIX normally virtualizes registry and AppData writes, which would hide those registrations from the browsers.

The manifest therefore declares `runFullTrust` and `unvirtualizedResources`, with exclusions limited to:

- `HKCU\Software\Google\Chrome\NativeMessagingHosts\app.timebridge.bridge`
- `HKCU\Software\Mozilla\NativeMessagingHosts\app.timebridge.bridge`
- `%APPDATA%\app.timebridge.desktop\browser-bridge`

Timer databases, settings, and all other application data remain virtualized. The precise exclusions make the native host discoverable without broadly disabling package virtualization. The first Store package intentionally targets Windows 11 (build 22000) or later so this security-sensitive integration can be tested against one modern packaging baseline.

Suggested Partner Center justification:

> Timebridge is a full-trust desktop timer service. It uses unvirtualizedResources only for two per-user browser native-messaging registration keys and the corresponding manifest directory in Roaming AppData. Chrome and Firefox must read these exact locations to connect the extension to the local timer service. No timer database, settings, or unrelated files or registry locations are excluded from virtualization.

## Reserve the app and get its identity

1. Create a Windows app in [Microsoft Partner Center](https://partner.microsoft.com/dashboard).
2. Reserve the name **Timebridge**.
3. Open **Product management > Product identity**.
4. Copy these values exactly:
   - Package/Identity/Name
   - Package/Identity/Publisher
   - Package/Properties/PublisherDisplayName

These values are public package metadata, not passwords or signing keys.

## Build the Store package

From the repository root:

```powershell
npm ci
npm run native:host
npm run desktop:build:prepared -- --no-bundle --no-sign
npm run store:msix -- -IdentityName 'PASTE_IDENTITY_NAME' -Publisher 'PASTE_PUBLISHER' -PublisherDisplayName 'PASTE_PUBLISHER_DISPLAY_NAME'
```

The output is `artifacts/Timebridge_<version>_x64.msix`. It is deliberately unsigned; upload it to Partner Center instead of distributing it directly. Microsoft signs the accepted Store package.

For local structural validation without a Partner Center identity:

```powershell
npm run store:msix -- -DevelopmentIdentity
```

That development package is not installable until it is signed with a locally trusted development certificate. Do not publish or distribute it.

## Acceptance test for the Store-signed package

After Partner Center produces a signed package:

1. Install it on a clean Windows 11 machine or VM.
2. Launch Timebridge once.
3. Install the Chrome or Firefox extension and verify it reports that the desktop service is connected.
4. Start a ten-second alarm from a supported web page.
5. Close the browser before the alarm expires.
6. Confirm that Timebridge displays the alarm and that Snooze and Dismiss work.
7. Restart the browser and confirm the extension reconnects.

Also test upgrade and uninstall behavior before moving the Store submission out of private or flighted testing.
