# @timebridge/sdk

Create local desktop alarms and timers from a browser or Node 22+ application. Requires Timebridge Desktop on the same computer and explicit user approval.

```js
import { createTimebridge } from '@timebridge/sdk';
const clock = createTimebridge({ appName: 'Example app' });
await clock.requestPermission();
await clock.createCountdown({ durationSeconds: 10 });
```

Native Node clients must also pass `origin: 'https://example.com'`. Optional alarm icons are PNG data URLs up to 16 KiB and 128 × 128 pixels. Browser `iconFromFile(blob)` helps prepare a user-selected image. Browser clients prefer the extension when activated on the page, otherwise the local HTTP bridge; browsers may require local-network permission.

This package is distributed as a release tarball; no npm registry publication is claimed. [Full documentation](https://github.com/Walltumbler/Time-Bridge/blob/main/docs/sdk.md) · [Source](https://github.com/Walltumbler/Time-Bridge). Apache-2.0.
