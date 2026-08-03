# Broker Keychain ACL — when the Mac login-password dialog appears

Service: `in.tradeautopsy.station.broker-credentials`  
(This is **not** Touch ID. Touch ID is only for `kotak-login-profile` on **Edit**.)

## When it can appear (at most once per grant)

macOS shows *“tradeautopsy-agent wants to use your confidential information…”* when **a different process** first accesses a Keychain item it did not create, and the item is not yet granted to that app.

| Situation | Prompt? | What to click |
|-----------|---------|----------------|
| Kotak mint → Start in same agent process | **No** (agent owns item + in-memory cache) | — |
| Brokers list refresh after Kotak Connect | **No** (Station uses metadata, not SecItem) | — |
| COM/HMAC Connect (Station writes vault) then agent Start | **Once** until Always Allow | **Always Allow** + Mac login password |
| Old Station-owned Kotak leftover, agent first touch | **Once** | **Always Allow**, then Delete → Connect again |
| Login profile Edit | Touch ID / passcode (not this dialog) | Fingerprint |

## What we changed so it is not 4×

Previously each of these could hit Keychain separately: Station post-mint read, Brokers `hasCredentials`, agent `present`, agent Start `load`.

Now:

1. **Agent vault cache** — mint `save` fills memory; `present` + Start reuse it (no re-prompt in-process).
2. **Station never reads Kotak `broker-credentials`** — verify via agent `present`; Brokers “configured” via metadata.
3. **COM** — Station writes with trusted-app ACL for Station + bundled agent; still use **Always Allow** the first time if macOS asks.

## Dogfood rule

If the password dialog appears: enter Mac login password → **Always Allow** (not Allow). After that it should not repeat for that item until the app binary path changes or the item is recreated by the other process.
