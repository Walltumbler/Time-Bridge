# Local JSON protocol v1

POST JSON to `http://127.0.0.1:47832/v1` on the same computer as Timebridge Desktop. Use `Content-Type: application/json` and a valid exact `Origin`. A native client must supply Origin; a browser supplies its own. HTTPS origins and localhost-only HTTP development origins are supported. The Host must remain exactly `127.0.0.1:47832`.

1. Send `{"op":"detect"}`. This read-only operation needs no credential.
2. Generate a cryptographically random 32-byte value, encoded as 64 hexadecimal characters, as `pollSecret`.
3. Send `{"op":"permissions.request","name":"Example app","pollSecret":"<secret>"}`. Store its `id` in memory. Ask the user to approve in Desktop.
4. Poll `{"op":"permissions.poll","id":"<id>","pollSecret":"<secret>"}` every 1.5 seconds, for at most two minutes.
5. On `status: approved`, retain the returned `credential` securely. Denied or expired requests must be surfaced to the user. A completed poll is single-use; do not run duplicate polling loops for one request.
6. Send `Authorization: Bearer <credential>` for subsequent operations. A credential is bound to the exact origin and cannot access another app's items.

Create a timer:

```json
{"op":"item.create","input":{"kind":"countdown","durationSeconds":10}}
```

Create a scheduled alarm (supply a future timestamp):

```json
{"op":"item.create","input":{"kind":"alarm","fireAt":"2030-01-01T09:00:00Z","timezone":"UTC","title":"Example reminder","icon":"data:image/png;base64,<small-PNG>"}}
```

Omit `icon` rather than sending the placeholder above. Icons must be valid static PNGs, at most 16 KiB and 128 × 128 pixels. Omit `title` for an automatic name. Optional `externalId` correlates an item with your app; it is not a duplicate-prevention key. Optional `returnUrl` must match the source origin. Never send a `source` field; Desktop assigns provenance.

Other operations:

| Operation | Fields |
| --- | --- |
| `permissions.status` | none |
| `items.list` | none |
| `item.get` | `id` |
| `item.update` | `id`, `title` |
| `item.action` | `id`, `action` |
| `events.listUndelivered` | none |
| `events.acknowledge` | `ids`: up to 100 event IDs |

Kinds and options are documented in the [SDK guide](sdk.md); the Rust `Rpc` and `CreateItem` definitions are the strict protocol schema. Unknown fields are rejected. Total JSON request size is limited to 32 KiB. Native-messaging envelopes also have a 32 KiB limit, so keep requests below that after origin and credential overhead. Errors use HTTP status codes and `{"error":"message"}`. Rate limits currently allow 120 requests per origin per minute, with tighter pairing limits, and a global limit. Use event polling sparingly.

Never automatically resend writes after an uncertain timeout. Reconnect and inspect the application's items. Never log a pairing secret or credential. Revocation is controlled by the user in Desktop Settings. Return links and icons do not grant an app any additional permissions.
