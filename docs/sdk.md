# SDK integration

Timebridge runs on the user's computer. Browser code and local Node/native apps can create alarms after the user approves the application's origin in Desktop. Server-side code on a remote machine cannot directly reach a user's local Timebridge.

## Install the SDK

The package is not yet published to npm. Download `timebridge-sdk-0.2.0.tgz` from a GitHub release, then:

```sh
npm install ./timebridge-sdk-0.2.0.tgz
```

From source, run `npm ci` and `npm run extensions:build` in the repository. Import `extensions/sdk/dist/index.js`, or package it with `npm pack ./extensions/sdk`. The package includes JavaScript ESM and TypeScript declarations. Plain HTML pages can copy the complete SDK `dist` directory and use `<script type="module">`; keep all four JavaScript modules together.

## Browser quick start

```js
import { createTimebridge, iconFromFile } from '@timebridge/sdk';

const clock = createTimebridge({ appName: 'Example app' });

// Run this from a Connect button, not automatically on page load.
async function connect() {
  const status = await clock.connect();
  if (!status.desktop) throw new Error('Open Timebridge Desktop first.');
  await clock.requestPermission(); // Ask the user to approve in Desktop.
}

async function remindMe(file) {
  const icon = file ? await iconFromFile(file) : undefined;
  const alarm = await clock.createCountdown({
    durationSeconds: 60,
    // title is optional; Desktop generates a label when absent.
    icon,
    externalId: 'task-123',
    returnUrl: location.href,
  });
  return alarm.id;
}
```

The bundled demo at Desktop **Settings → Open setup guide & test alarm** implements this flow and creates a 10-second timer. `npm run demo` serves the same example on `http://127.0.0.1:4790` during development.

Use HTTPS in production. HTTP is permitted only for exact `localhost` or `127.0.0.1` development origins, including the port. Origins are independent: `http://localhost:4790` and `http://127.0.0.1:4790` need separate approval.

With the extension installed, open its toolbar popup on the current page to activate the bridge. Automatic detection prefers the extension, then attempts the local bridge. Some browsers require local-network permission for direct access or block it under page policy; use the extension in that case. A page's CSP must permit its chosen connection mechanism and the SDK's own module files.

`createTimebridge({ transport: 'extension' })` forces the extension for testing; `'loopback'` forces direct HTTP. Detection alone does not grant permission. The extension stores its credential; direct browser credentials live in memory by default, so a page reload requires reapproval unless the app explicitly implements secure credential handling.

## Node and native applications

```js
import { createTimebridge } from '@timebridge/sdk';

const clock = createTimebridge({
  appName: 'Example desktop app',
  origin: 'https://example.com',
  transport: 'loopback',
  // credential: await keychain.read('timebridge'),
  // onCredential: value => keychain.write('timebridge', value),
});
try {
  await clock.requestPermission();
  await clock.createAlarm({
    fireAt: new Date(Date.now() + 60000).toISOString(),
    timezone: 'UTC',
  });
} finally { clock.dispose(); }
```

Use a stable HTTPS origin identifying your app. It is a namespace shown during approval, not a claim that the process has proved ownership of that domain. Use distinct origins for distinct apps; approval of the same origin rotates that origin's credential. Node 22+ supplies fetch and Web Crypto. Never log credentials or store them in URLs or repository files.

Run `node examples/node/alarm.mjs` or `python examples/python/alarm.py` with Desktop open for working local examples. For C#, Java, Rust, or other languages, see the [JSON protocol](protocol.md). The Python example uses only the standard library.

## Icons

Every creation method accepts `icon?: string`. Send a **static PNG data URL**, at most **16 KiB decoded**, no larger than **128 × 128 pixels**. `iconFromFile(blob)` produces a 64-pixel PNG in a browser. Native callers can base64-encode an appropriately sized PNG. Omit the field to use Timebridge's standard kind icon.

Remote URLs, SVG, animated PNG, and malformed images are rejected. Icons stay local, appear on cards and ringing alerts, survive restart, and are copied when snoozing. They are visual labels, not verified app identities; Desktop still shows the source origin. Native OS toast notifications currently use the Timebridge app identity.

## API

All methods are asynchronous except `dispose`. Create methods return the persisted item, including its stable `id`.

| Method | Arguments / behavior |
| --- | --- |
| `connect()` | Returns `desktop`, `extension`, `transport`, and protocol/version capabilities when available. |
| `requestPermission()` | Reuses valid approval or requests it; expires after two minutes. Concurrent calls share one request. |
| `createCountdown(options)` | `durationSeconds` between 1 and 31,536,000. |
| `createRangedCountdown(options)` | `minimumSeconds`, `maximumSeconds`, with maximum greater than minimum. |
| `createAlarm(options)` | `fireAt`: ISO timestamp with offset; `timezone`: IANA name. |
| `createWeeklyAlarm(options)` | `weekdays`: Monday=0 to Sunday=6; `time`: `HH:mm`; `timezone`. |
| `createDateListAlarm(options)` | `occurrences`: ISO timestamps with offsets; `timezone`. |
| `createStopwatch(options?)` | Starts a persistent stopwatch. |
| `listItems()` / `getItem(id)` | Only the approved application's items. |
| `updateItem(id, title)` | Renames the item; does not reschedule it. |
| `actOnItem(id, action)` | `pause`, `resume`, `cancel`, `done`, `dismiss`, `silence`, `extend1`, `extend5`. |
| `listUndeliveredEvents()` | Events for this application's origin that it has not acknowledged. |
| `acknowledgeEvents(ids)` | Acknowledge up to 100 positive event IDs after processing. |
| `dispose()` | Removes the browser listener and rejects outstanding extension calls. |

Creation options share optional `title`, `icon`, and `externalId`; timer/alarm options also accept `returnUrl`. Browser timer/alarm methods default the return URL to the current page. Native methods omit it unless provided. Return URLs must stay on the exact approved origin. An external ID is correlation metadata, not an idempotency key.

Handle `TimebridgeError.code`: `PAIRING_REQUIRED`, `PAIRING_DENIED`, `PAIRING_EXPIRED`, `DESKTOP_UNAVAILABLE`, `EXTENSION_TIMEOUT`, `RATE_LIMITED`, `INVALID_RETURN_URL`, or `BRIDGE_ERROR`. Show the accompanying message. Timeouts can happen after a write succeeds: reconnect and inspect items before retrying. The SDK never retries a mutation over another transport automatically.

## Events and lifecycle

Poll events no more frequently than every five seconds during normal operation. Persist the work you do for an event, then acknowledge its ID. Delivery is at least once; consumers must tolerate repeats. Timer scheduling and ringing do not depend on browser event polling. Closing the page or exiting the client does not delete its timers. Revocation prevents further access but leaves timers available to the user in Desktop.

Alarms require Desktop to be running and the computer awake. Websites cannot modify the user's output-device, volume, mute, or notification preferences.
