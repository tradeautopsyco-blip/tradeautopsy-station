# PRD - Backend Box v1: Binance.US Broker Sync Control Plane

**Status:** Ready for agent  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Parent context:** TradeAutopsy Station v1 (`FExEVIL/tradeautopsy` #161)  
**Grill-me:** Complete (31 decisions, 2026-06-29)  
**Scope:** Brokers screen in Station window, Binance.US first, macOS Keychain credentials, broker sync control, connection status/logs, internal Environments control

---

## Problem Statement

TradeAutopsy Station has a Phase 0 shell with a Brokers route placeholder, but traders still have no safe way to connect a real broker from the Station window. Without Backend Box v1:

1. Traders cannot enter Binance.US credentials in the native app and have Station validate, store, and use them safely.
2. Broker sync is not controllable at runtime; today the agent can be launched with a broker adapter, but there is no user-facing Start, Stop, Delete, or degraded-state contract.
3. Behavioral intelligence cannot reliably receive broker activity metadata from Station because connection identity, redaction, opt-out, and completeness rules are not defined.
4. The UI can mislead traders if it shows "connected" while fills, balances, or open orders are stale, partially synced, rate limited, or blocked by the local agent being offline.
5. Internal QA needs dev/staging/prod target switching for backend and behavioral-event validation, but normal production traders must not be able to misroute real broker data.

Backend Box v1 must turn the Brokers screen into a safe broker sync control plane, not a broker operations console.

---

## Solution

Ship the Station **Brokers** screen as Backend Box v1, focused on Binance.US crypto sync.

North-star acceptance statement:

> A trader can connect Binance.US once, have Station validate and securely store credentials, auto-start read sync, see honest sync health, pause/resume/delete the connection, and feed redacted broker activity into behavioral intelligence unless opted out.

The product flow is low friction:

1. The trader opens Brokers.
2. The trader enters Binance.US API credentials.
3. Station performs local field checks, then a live Binance.US validation.
4. If validation succeeds and withdraw permission is not detected, Station saves credentials to macOS Keychain.
5. Sync starts automatically after successful validation.
6. The broker card shows honest runtime state: syncing, degraded, rate limited, paused, failed, unavailable, or not configured.
7. The trader can Stop, Start, or Delete the connection.

Backend Box v1 keeps the local agent running. Stop pauses broker sync only; it does not restart or terminate the agent. Kill switch, audit log, and SSE remain unaffected.

---

## User Stories

### Connecting Binance.US

1. As a crypto trader, I want to connect Binance.US from the Brokers screen, so that TradeAutopsy can use my broker activity without leaving Station.
2. As a trader, I want Binance.US to be the only enabled broker in v1, so that the first release focuses on one crypto broker path.
3. As a trader, I want disabled future broker cards to make clear that multi-broker support is planned later, so that I understand why other asset classes are not available yet.
4. As a trader, I want clear API key and secret fields, so that I know exactly what credentials Station needs.
5. As a trader, I want Station to validate my credentials with Binance.US before saving them, so that saved credentials mean a usable connection.
6. As a trader, I want failed validation to explain whether credentials, permission, network, rate limit, or Binance availability caused the problem, so that I know what to fix.
7. As a trader, I want transient validation failures to keep typed values in memory during the session, so that I do not have to retype keys after a network or Binance outage.
8. As a trader, I want transient validation failures not to persist credentials to Keychain, so that Keychain only stores validated connections.
9. As a trader, I want successful validation to save credentials and start sync automatically, so that connecting is a single low-friction flow.
10. As a trader, I want the save screen to disclose that connecting starts broker sync and behavioral/account metadata upload unless I opt out, so that I understand the data flow before connecting.

### Credential Safety

11. As a security-conscious trader, I want credentials stored in macOS Keychain, so that secrets are not saved in app preferences, logs, or local plaintext files.
12. As a trader, I want Station to block saving if withdraw permission is detected, so that an unnecessary catastrophic permission is never accepted.
13. As a trader, I want trade-enabled keys to be allowed with a persistent warning, so that v1 remains low friction while still telling me read-only keys are safer.
14. As a trader, I want unverifiable key permissions to be allowed with a persistent warning, so that I can connect when Binance.US permission introspection is incomplete.
15. As a trader, I want read-only confirmed keys to appear healthy, so that I can tell I used the safest key type.
16. As a trader, I want the agent to read credentials fresh from Keychain on Start, so that it does not keep long-lived plaintext secrets at rest.
17. As a trader, I want Station to avoid repeated Keychain prompts where possible, so that security does not make normal use frustrating.
18. As an engineer, I want no API key, secret, session token, HMAC signature, auth header, cookie, signed URL, or raw auth-bearing payload to reach logs, SSE, UI models, or brain uploads, so that secret leakage is structurally forbidden.

### Runtime Sync Control

19. As a trader, I want Stop to pause broker sync immediately, so that I can stop broker polling without quitting Station.
20. As a trader, I want Stop to avoid restarting the agent, so that kill switch, audit log, SSE, and other agent features continue running.
21. As a trader, I want Stop to stop fill polling immediately, so that the broker adapter no longer talks to Binance.US.
22. As a trader, I want Stop to keep credentials in Keychain, so that I can resume later without re-entering them.
23. As a trader, I want Stop to keep the last known kill-switch broker slug unchanged, so that pausing sync does not rewrite runtime safety context.
24. As a trader, I want Notch live P&L to freeze and then degrade to unavailable after the stale threshold, so that I am not misled by stale broker data.
25. As a trader, I want the pulse strip to turn amber degraded when sync is stopped or stale, so that broker health is visible outside the Brokers screen.
26. As a trader, I want Start to resume sync with credentials pulled fresh from Keychain, so that restarting sync does not reuse stale plaintext secrets.
27. As a trader, I want manual Stop to persist Paused across app restart, so that Station does not override my explicit pause.
28. As a trader, I want auto-start only after successful connect/validation, so that first connection is low friction but later manual pauses are respected.

### Delete / Remove Connection

29. As a trader, I want Delete to require confirmation, so that I do not accidentally remove a saved broker connection.
30. As a trader, I want the confirmation to say Delete removes saved Keychain credentials and stops broker sync, so that I understand the consequence.
31. As a trader, I want Delete to stop sync immediately before removing credentials, so that no broker polling continues after removal.
32. As a trader, I want Delete to remove the Binance.US Keychain item, so that this Mac no longer has saved credentials for that connection.
33. As a trader, I want Delete to clear saved validation status and permission warnings, so that removed connections do not keep stale health UI.
34. As a trader, I want Delete to clear cached broker runtime UI state, so that the card returns to Not configured.
35. As a trader, I want Delete not to erase historical fills and trades already ingested, so that trading history remains available for journal and analysis.
36. As a trader, I want Delete not to rewrite past audit records, so that audit history remains honest.
37. As an engineer, I want redacted debug and behavioral history to remain available after Delete, so that support and intelligence are not blind to past sync behavior.

### Broker Card Status

38. As a trader, I want a Not configured state, so that I know no credentials are saved.
39. As a trader, I want a Validating state, so that I know Station is checking credentials with Binance.US.
40. As a trader, I want a Ready to Start state when credentials exist but sync is not running, so that I know a connection is configured.
41. As a trader, I want a Syncing state only when all required data classes are current, so that "connected" never hides partial failure.
42. As a trader, I want Degraded to appear when credentials work but one or more required data classes are failing or stale, so that partial sync is visible.
43. As a trader, I want Rate Limited to appear separately from generic failure, so that I know Binance.US throttling is the current blocker.
44. As a trader, I want Paused to appear after manual Stop, so that I can distinguish my action from a failure.
45. As a trader, I want Unavailable: Agent Offline when credentials exist but the local agent is down, so that I do not mistake an agent outage for a broker problem.
46. As a trader, I want Failed for auth failure, missing required access, withdraw permission, or total sync failure, so that unrecoverable states are clear.
47. As a trader, I want status details to name the failing data class where possible, so that "trade history delayed, balances live" is visible.

### Data Synced in v1

48. As a trader, I want v1 to sync fills and trade history, so that TradeAutopsy can understand executed trading behavior.
49. As a trader, I want v1 to sync balances and holdings, so that TradeAutopsy can understand exposure.
50. As a trader, I want v1 to sync open orders, so that TradeAutopsy can understand active intent and risk.
51. As a trader, I want v1 to defer deposits and withdrawals, so that the first release does not pull money-movement history.
52. As a trader, I want fees included only when present in fill or trade payloads, so that v1 does not add a separate fee-sync surface.
53. As a trader, I want unrealized P&L shown only when reliably derivable, so that Station does not invent precision from incomplete cost basis.
54. As a trader, I want first connect to backfill 90 days of fills and trade history, so that behavioral intelligence has useful recent context without pulling years of sensitive history.
55. As a trader, I want balances and open orders treated as current snapshots, so that current exposure is not confused with historical event streams.
56. As a trader, I want ongoing trade-history sync to be incremental from the last seen fill timestamp, so that Station avoids duplicate and excessive polling.

### Partial Sync and Retry

57. As a trader, I want incomplete first sync to show Degraded / Partial Sync rather than Connected, so that the card is honest.
58. As a trader, I want partial data to be retained when useful, so that successful data classes are not thrown away because another class failed.
59. As an intelligence consumer, I want partial uploads to include completeness flags for each data class, so that downstream analysis does not treat partial data as complete.
60. As a trader, I want failed data classes retried automatically with backoff, so that transient failures recover without manual work.
61. As a trader, I want successful data classes to stay fresh while failed classes retry, so that one broken endpoint does not stop all broker insight.
62. As a trader, I want retry eventually to stop hammering Binance.US and ask for manual Retry, so that Station respects broker limits and avoids endless silent churn.
63. As a trader, I want rate-limit state to show when Station will retry, so that I know whether action is needed.
64. As an engineer, I want rate-limit events uploaded as normalized behavioral events, so that behavior analysis can include throttling without raw broker payloads.

### Behavioral Intelligence and Privacy

65. As a trader, I want broker connection events to feed behavioral intelligence by default, so that TradeAutopsy can learn from broker-connection behavior.
66. As a trader, I want an explicit opt-out for behavioral analysis, so that I can disable behavioral uploads while still using local broker sync.
67. As a trader, I want sync stopped, validation failed, permission warning ignored, stale for 60s, manual restart, rate limited, and partial sync events to be eligible behavioral signals, so that the system can learn from friction and avoidance patterns.
68. As an intelligence consumer, I want broker events and synced records to include broker connection identity, broker slug, asset class, and environment, so that future multi-broker data is never merged incorrectly.
69. As a security-conscious trader, I want raw broker request bodies, response bodies, and error bodies forbidden from upload, so that broker payloads are never copied wholesale into TradeAutopsy.
70. As a security-conscious trader, I want raw auth material forbidden from upload under all circumstances, so that behavioral analysis cannot override secret redaction.
71. As an intelligence consumer, I want normalized broker/account activity metadata allowed when behavioral analysis is enabled, so that balances, holdings, order IDs, precise timestamps, device/IP identifiers, and unsafe-warning behavior can inform patterns.
72. As an engineer, I want every upload path to run through a redaction boundary before leaving the agent, so that unredacted payloads cannot bypass policy.
73. As support, I want redacted local connection logs to preserve useful context after Delete, so that user issues can be diagnosed without secrets.

### Environments

74. As an internal developer, I want dev, staging, and prod environments, so that I can test Station against non-production TradeAutopsy backends.
75. As QA, I want staging to receive broker sync and behavioral events before release, so that we can validate schemas without contaminating production.
76. As an engineer, I want dev to support local or unstable backend testing, so that feature work does not depend on production services.
77. As a production trader, I do not want to see environment switching, so that I cannot accidentally route real broker data to staging or dev.
78. As an internal user, I want environment switching gated to internal/dev builds or an internal unlock, so that only authorized testers can change targets.
79. As an internal tester, I want Binance.US credentials isolated per environment, so that staging credentials do not silently appear in prod and prod credentials do not appear in dev.
80. As an internal tester, I want environment switch to require confirmation when sync is running, so that target changes are intentional.
81. As an internal tester, I want environment switch to stop broker sync, reconnect the agent to the selected TradeAutopsy backend, clear environment-scoped runtime UI state, and require Start again, so that active sync sessions do not carry across targets.

### Agent Offline and Source of Truth

82. As a trader, I want Backend Box to show Unavailable: Agent Offline when credentials exist but the agent is down, so that I know the local agent is the blocker.
83. As a trader, I want non-secret metadata visible while the agent is offline, so that I can still see broker name, last validated time, and last sync summary.
84. As a trader, I want Start and Stop disabled while the agent is offline, so that I do not issue controls the agent cannot execute.
85. As a trader, I want Delete available while the agent is offline only if Station can safely delete Keychain credentials directly, so that I can remove secrets without waiting for agent recovery.
86. As an engineer, I want the local agent to be source of truth for runtime sync state, so that Station UI never pretends a broker is connected when the agent disagrees.
87. As an engineer, I want Keychain to be source of truth for secrets, so that credentials are not duplicated in the UI or brain.
88. As an engineer, I want the brain to be source of truth for uploaded behavioral/history records, so that local runtime status and server history stay separate.
89. As an engineer, I want Station UI to be only a presenter/controller, so that state ownership is testable and not split across views.

### Testing and Release

90. As QA, I want a fake/local Binance.US adapter for automated tests, so that validation, sync, Stop, Start, Delete, degraded, rate-limit, and offline states can be tested without real secrets.
91. As QA, I want a manual real Binance.US smoke checklist using the founder account, so that the shipped path is proven against the real broker.
92. As an engineer, I want CI never to depend on real Binance.US credentials, so that tests are deterministic and safe.
93. As a release owner, I want withdraw-permission hard block proven with a fake adapter, so that the most important permission guard is not left to manual testing.
94. As a release owner, I want no raw broker or auth payload in logs or uploads, so that release cannot ship with a secret-leak path.
95. As a release owner, I want Stop/Start proven without agent restart, so that runtime control is real and not a disguised process reset.
96. As a release owner, I want Delete proven to remove Keychain credentials, so that credential removal is trustworthy.

---

## Implementation Decisions

### Product Scope

- Backend Box v1 is the **Brokers** screen content inside the existing Station shell.
- Binance.US is the first and only enabled broker in v1.
- Crypto-only is locked for v1.
- The data model must support multiple broker connections from day one, but the v1 UI enables only Binance.US.
- Future broker cards may appear disabled or planned, but they must not be connectable in v1.
- Backend Box v1 is a broker sync/control plane, not a broker operations console.

### Core State Ownership

- The local agent is the source of truth for runtime broker sync state.
- macOS Keychain is the source of truth for secrets.
- The TradeAutopsy brain is the source of truth for uploaded behavioral/history records.
- Station UI is a presenter/controller and must not invent connected state independently from the agent.

### Broker Connection Identity

Every broker event and synced record must carry identity sufficient for future multi-broker, multi-asset-class support:

- `broker_connection_id`
- `broker_slug`
- `asset_class`
- `environment`

This applies even when v1 has only one Binance.US connection.

### Credential Lifecycle

- Credentials are saved only after live Binance.US validation succeeds.
- Local field validation is necessary but not sufficient for persistence.
- Transient validation failures can keep typed values in memory during the current UI session, but must not persist to Keychain.
- Successful validation saves credentials and auto-starts sync.
- Station can write/delete credentials and validate during setup.
- The agent reads credentials fresh from Keychain on Start.
- No long-lived plaintext secrets should live in Station state or agent state at rest.
- No secrets may be sent through logs, SSE, UI models, behavioral uploads, or brain calls.

### Permission Policy

- Withdraw permission detected: hard block; do not save; do not start.
- Trade/order permission detected: allow with persistent warning.
- Permissions unverifiable: allow with persistent warning.
- Read-only confirmed: healthy credential posture.

This is intentionally less compliant than a strict read-only-only release, but keeps one non-negotiable safety line around withdraw capability.

### Auto-Start and Manual Pause

- After successful validation, sync starts automatically to minimize user friction.
- The connection flow must disclose that broker sync and behavioral/account metadata uploads start after connect unless behavioral analysis is opted out.
- Manual Stop pauses broker sync immediately and persists Paused.
- App restart after manual Stop must not auto-resume sync.
- Auto-start applies to the first successful connect/validation flow, not to overriding a later user pause.

### Stop / Start Semantics

- Stop pauses the broker sync adapter immediately.
- Stop requires no agent restart.
- Stop does not terminate the agent process.
- Fill polling stops immediately.
- Kill switch, audit log, SSE, and other agent behavior remain unaffected.
- Notch live P&L freezes, then degrades to unavailable after the stale threshold.
- Pulse strip goes amber degraded.
- The last known kill-switch broker slug is unchanged by Stop.
- Credentials stay in Keychain.
- Start resumes broker sync using credentials freshly read from Keychain.

### Delete / Remove Semantics

- Delete requires confirmation.
- Confirmation must explicitly say saved Keychain credentials will be removed and broker sync will stop.
- Delete stops sync immediately before removing credentials.
- Delete removes the Binance.US Keychain item.
- Delete clears saved validation status, permission warnings, connected-card state, and runtime UI cache.
- Delete does not erase historical fills/trades already ingested.
- Delete does not rewrite historical audit records.
- Redacted connection/debug history may remain for support and behavioral analysis.

### Broker Card Status Taxonomy

The v1 status set is:

- `Not configured`
- `Validating`
- `Ready to Start`
- `Syncing`
- `Degraded`
- `Rate Limited`
- `Paused`
- `Unavailable: Agent Offline`
- `Failed`

Connected/syncing must mean all required data classes are current enough. Partial success must not display as fully connected.

### Data Classes

Backend Box v1 syncs:

- Fills/trade history
- Balances/holdings
- Open orders

Backend Box v1 defers:

- Deposits
- Withdrawals

Fees are included only when present in fill/trade payloads. Unrealized P&L is shown only when reliably derivable from available data; otherwise it must be unavailable rather than guessed.

### Backfill and Incremental Sync

- First connect performs a 90-day fills/trade-history backfill.
- Balances and open orders are current-state snapshots.
- Ongoing fills/trade-history sync is incremental from the last seen fill timestamp.

### Partial Sync

- If first sync partially succeeds, the broker card shows Degraded / Partial Sync, not Connected.
- Partial data can be uploaded and used only with explicit completeness flags.
- Completeness flags must cover at least fills/trade history, balances/holdings, and open orders.
- Successful data classes should remain fresh while failed classes retry.
- Failed data classes retry automatically with backoff.
- After a fixed retry window or circuit threshold, Station remains Degraded and asks for manual Retry instead of retrying forever.

### Rate Limit

- Binance.US rate limiting is a first-class degraded state.
- Station slows polling/backfill automatically when rate limited.
- The card shows a retry time when known.
- Already synced data remains available with freshness/completeness metadata.
- Behavioral upload uses normalized `broker_rate_limited` style events, not raw Binance payloads.

### Behavioral Intelligence

- Broker connection logs are behavioral data, not only UI diagnostics.
- Behavioral upload is enabled by default.
- Users can explicitly opt out of behavioral analysis.
- Sync may still run when behavioral analysis is opted out; behavioral uploads stay off.
- Eligible behavioral events include sync stopped, validation failed, unsafe permission warning ignored, stale threshold reached, manual restart, rate limited, partial sync, and retry behavior.
- Partial data uploads must include completeness flags.

### Redaction Boundary

Never upload under any circumstance:

- API keys
- API secrets
- Passphrases
- Session tokens
- OAuth tokens
- Cookies
- HMAC signatures
- Auth headers
- Signed URLs
- Raw Binance request bodies
- Raw Binance response bodies
- Raw broker error bodies
- Keychain item identifiers that expose secret names or account structure
- macOS username
- Local file paths
- Machine serial number
- Any crash/log blob before redaction has run

Allowed when behavioral analysis is enabled and redaction has run:

- Normalized balances
- Holdings
- Order IDs
- Precise timestamps
- Device/IP identifiers
- Unsafe-warning behavior
- Normalized error class/code and sanitized message category
- Retry/backoff/failure metadata

### Environments

- Environments exist for internal/dev/QA workflows: non-production backend testing, broker sync validation, behavioral event validation, issue reproduction, and keeping production data clean.
- Normal production users always use prod and should not see environment switching.
- Environment switching is internal-only, either build-gated or behind an internal unlock.
- Environments isolate TradeAutopsy backend target and behavioral upload destination.
- Binance.US remains real/live unless a developer explicitly uses mock credentials/adapters.
- Keychain credentials are isolated per environment.
- Environment switch while sync is running requires confirmation.
- Confirmed environment switch stops broker sync, reconnects the agent to the selected TradeAutopsy backend, clears environment-scoped runtime UI state, and requires Start again.

### Agent Offline Behavior

- If credentials exist but the agent is down, the broker card shows `Unavailable: Agent Offline`.
- Non-secret saved metadata may remain visible: broker name, last validated time, last sync summary.
- Start and Stop are disabled while the agent is offline.
- Delete can remain available only if Station can safely remove Keychain credentials directly without agent coordination.

### Release Blockers

Backend Box v1 cannot ship unless all of these are true:

- Successful real Binance.US validation and auto-start sync are proven manually.
- Stop/Start works without agent restart.
- Delete removes Keychain credentials.
- Withdraw-permission hard block is proven through fake/local adapter tests.
- Behavioral uploads respect opt-out.
- Redaction rules are enforced for logs and uploads.
- No raw broker/auth payload appears in logs, SSE, UI models, or uploads.

---

## Testing Decisions

### Primary Seam

The preferred highest seam is the **broker connection control seam between Station and the local agent**:

- Station drives connect, validate, auto-start, Stop, Start, Delete, environment switch, and agent-offline presentation through a broker-control client/view-model boundary.
- The local agent owns runtime sync state, broker adapter lifecycle, Keychain reads, data-class completeness, and behavioral upload/redaction.
- Tests should assert observable states and side effects at this boundary, not private UI method order.

This keeps UI tests thin and prevents individual SwiftUI views from becoming the primary test surface.

### Agent Test Seam

- Extend the existing broker adapter boundary with a fake/local Binance.US adapter capable of deterministic validation, permission posture, data-class success/failure, rate-limit, and malformed/unsafe scenarios.
- Existing broker sync tests and fake adapter patterns are prior art.
- Agent tests should prove Stop/Start without process restart, partial sync completeness flags, rate-limit state, retry/backoff, and redaction before upload.

### Station Test Seam

- Add or extend Station tests around an injectable broker-control client/view-model.
- Tests should cover card status taxonomy, auto-start after validation, persistent Paused after Stop, Delete confirmation effects, agent-offline state, environment gating, and behavioral opt-out presentation.
- Tests should use fakes instead of real Keychain or Binance.US.

### Keychain Test Strategy

- Unit tests should use an injectable Keychain abstraction/fake for save/read/delete behavior.
- A macOS integration or manual smoke path can verify real Keychain behavior.
- Tests must assert that Delete removes credentials and that Start reads fresh credentials from the credential store.

### Behavioral Upload and Redaction Tests

- Tests must assert the never-upload list.
- Tests must include raw broker error bodies and raw request/response payloads to prove they are not emitted.
- Tests must assert opt-out suppresses behavioral uploads while sync can continue.
- Tests must assert completeness flags on partial sync uploads.

### Manual Real Binance.US Smoke

Manual smoke uses the founder's Binance.US account and is never required in CI. It must cover:

1. Add credentials.
2. Live validation succeeds.
3. Successful validation auto-starts sync.
4. Fills/trade history backfill starts with 90-day scope.
5. Balances/holdings snapshot appears.
6. Open orders snapshot appears, including empty-state if there are no open orders.
7. Stop pauses sync without agent restart.
8. Start resumes sync.
9. Delete removes Keychain credentials and returns card to Not configured.
10. Behavioral opt-out suppresses behavioral uploads while sync continues.

### CI Rule

CI must never require real Binance.US credentials.

---

## Out of Scope

- Broker order placement.
- Broker entry orders.
- Broker withdrawals or money movement.
- Deposits/withdrawals sync.
- Enabling multiple real broker connections in the v1 UI.
- Non-Binance.US broker auth flows.
- Non-crypto asset-class behavior in the v1 UI.
- User-editable environment switching in production.
- Advanced raw log export.
- Custom polling intervals.
- P&L guarantees when cost basis is incomplete.
- Windows/Linux support.
- Kill switch logic changes.
- Brain-side broker sync implementation beyond required behavioral/history ingest contracts.

These are post-v1 slices and should be shipped separately after Backend Box v1 proves the safe broker sync/control plane.

---

## Further Notes

- Phase 0 Station shell already owns the Brokers route placeholder. This PRD fills that screen with Backend Box v1 behavior.
- Existing architecture invariants still apply: Station/Notch call only the local agent; the agent owns egress to TradeAutopsy; Station performs no LLM inference.
- The current agent broker sync is adapter-based and already emits broker sync state. Backend Box v1 needs runtime control, Keychain-backed credentials, richer data classes, completeness metadata, and redacted behavioral upload semantics.
- Backend Box should stay low-friction for normal traders: connect once, validate, auto-start, and show honest state.
- Backend Box should stay high-friction only where risk is high: withdraw permission, Delete, environment switching, and raw/secret data handling.
