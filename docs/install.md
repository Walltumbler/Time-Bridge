# Install and connect

## Windows desktop

1. Open [Timebridge Releases](https://github.com/Walltumbler/Time-Bridge/releases).
2. Download the latest `Timebridge_*_x64-setup.exe`, run it, and complete setup. The installer includes the browser helper and installs the WebView2 runtime if needed.
3. Open Timebridge. The first-run screen offers a guided connection test. You can also open it from **Settings → Open setup guide & test alarm**.
4. Keep Desktop running in the tray. Closing its window does not stop timers; choosing Quit does.

Only download release installers whose GitHub release includes checksums and build provenance. Development builds may be unsigned; public releases are expected to carry a valid Authenticode publisher signature. If the repository has no published release yet, build from source using the README. Do not treat a ZIP containing extension files as a desktop installer.

## Connect a website or application

Click its **Connect to Timebridge** button, then approve the displayed origin in Desktop. Return to the app and create a timer. Names are optional. An integrated app may supply its own small icon.

For a first test, use the bundled guide. Choose Connect, approve **Timebridge Demo**, then create a 10-second alarm. The timer should appear immediately in Desktop with your chosen icon and ring even if you close the browser page.

## Browser extensions

Direct local connections can work without an extension. If your browser blocks that connection, install the Timebridge extension and open its toolbar popup on the website you want to connect. Only the selected page is activated; new tabs or reloads may require clicking the extension again.

The ordinary release process is **Add to Chrome** from the Chrome Web Store and **Add to Firefox** from Mozilla Add-ons. Store links will appear in the setup guide when published. They are currently pending. Chrome requires Web Store distribution for normal Windows/macOS consumer installation; Firefox release builds require Mozilla-signed add-ons. The Firefox extension requires Firefox Desktop 140 or newer and is intentionally unavailable for Firefox Android because it depends on a desktop native-messaging host. A desktop installer cannot silently install either extension for the user.

### Development builds

Download the browser's extension ZIP from the matching GitHub release and extract it into a permanent folder.

- **Chrome:** open `chrome://extensions`, enable Developer mode, choose Load unpacked, and select the folder containing `manifest.json`. Use Reload on that page after replacing files.
- **Firefox:** open `about:debugging#/runtime/this-firefox`, choose Load Temporary Add-on, and select `manifest.json`. This temporary installation is removed when Firefox restarts.

Then open the website, select the Timebridge extension, and choose Connect this site. Approve in Desktop. Keep the website tab open during approval; the toolbar popup may close when you switch apps. Credentials stay in extension storage. Reopening the popup and choosing Connect on an already-approved site reuses that approval.

## Troubleshooting

| Symptom | Next step |
| --- | --- |
| Desktop not found | Open Timebridge; try the extension if direct local access is blocked. |
| Browser asks for local-network permission | Allow access to the local desktop service if you want direct access, or choose the extension. |
| Local browser helper unavailable | Open Desktop once after installing. Inspect Settings for a helper registration error, then restart the browser. Reinstall if the helper is missing. |
| No sound | In Desktop Settings choose the alarm output device, test sound, and optionally enable the 75% volume override. |
| Cannot hear headphones | Select speakers as the alarm output. The normal Windows output remains unchanged. |
| Site no longer permitted | Connect and approve again; it may have been revoked in Settings. |

Upgrade by quitting Timebridge and installing the newer build. Your timers and preferences remain in the application data directory. Uninstallation removes program files; keeping or deleting personal timer data is a separate user decision.

References: [Chrome distribution](https://developer.chrome.com/docs/extensions/how-to/distribute), [Firefox signing](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/), [Firefox desktop-app distribution](https://extensionworkshop.com/documentation/publish/distribute-for-desktop-apps/).
