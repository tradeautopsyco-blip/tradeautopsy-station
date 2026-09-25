# Plan: Station auto-update infrastructure (Sparkle 2)

> Reference model: Boring Notch (`Sparkle.framework` + GitHub Pages appcast + signed DMGs + optional Homebrew).  
> Goal: same **trust model** (EdDSA appcast, monotonic build numbers, stable/beta channels) for **TradeAutopsy Station**, without breaking wire-protocol or kill-switch invariants.

## Why this plan exists

Station today ships as **unsigned, ad-hoc** DMGs (`scripts/pack-station-dmg.sh`, CI artifact `TradeAutopsy-Station-app`, 7-day retention). There is **no in-app updater**, **no release workflow**, and **no feed**. Cofounders get Gatekeeper friction and manual re-install.

Sparkle is the right fit because Station is already a **native macOS `.app`** with a **Release DMG** shape—the same artifact Boring Notch publishes to GitHub Releases.

## Architectural decisions (durable)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Update engine | **Sparkle 2** (SPM pin, same major as Boring Notch) | Mature sandbox downloader/installer XPC, EdDSA, channels |
| Feed URL | **`https://<org>.github.io/tradeautopsy-station/appcast.xml`** (or `updates.tradeautopsy.in` later) | GitHub Pages from `updater/` on `main`; custom domain optional |
| Artifact | **Single `TradeAutopsy-Station.dmg`** per release (whole `.app` replace) | Bundled `tradeautopsy-agent` must stay in lockstep with UI/Notch |
| Version semantics | **`CFBundleShortVersionString`** = semver (`0.2.0`); **`CFBundleVersion`** = monotonic integer build | Sparkle compares `CFBundleVersion` to `<sparkle:version>` |
| Wire contract | Bump **`station-wire/v1.json`** `version` + release notes when hop tables change | README already requires pinning on every release |
| Signing (production) | **Developer ID Application** + **notarization** + Hardened Runtime | Required for Sparkle installs without Gatekeeper pain; unsigned path stays for internal dogfood only |
| Channels | **default (stable)** + **`beta`** (RC / dogfood) | Mirror Boring Notch: beta items tagged `<sparkle:channel>beta</sparkle:channel>` |
| Private key | **`PRIVATE_SPARKLE_KEY`** in GitHub Actions; **`SUPublicEDKey`** in `Info.plist` | One keypair; rotate only with coordinated app + appcast migration |
| Parallel path | **Homebrew cask** optional (`auto_updates` + `livecheck`) | For developers who `brew upgrade`; must not fight Sparkle |
| CI split | **`ci.yml`** = build/test only; **`release.yml`** = ship (like Boring Notch vs `cicd.yml`) | Avoid accidental publishes on every push |

## Current state (baseline)

| Area | Today |
|------|--------|
| Bundle ID | `in.tradeautopsy.station` |
| Version | `0.1.0` / build `1` (`station/StationApp/Info.plist`) |
| Sparkle | Not integrated |
| Entitlements | No `.entitlements` file in repo (not sandboxed yet) |
| Pack | `pack-station-dmg.sh` — arm64, ad-hoc, embeds release agent |
| CI | Builds Debug `.app`, packs unsigned DMG, uploads artifact |
| Releases | Manual Drive / artifact / ad-hoc `gh release` |

## Gaps vs Boring Notch (must close)

1. **Code signing + notarization** — Sparkle can run unsigned in dev, but production updates need trusted replacement of `/Applications/TradeAutopsy Station.app`.
2. **Sparkle in Xcode** — SPM dependency, `SPUStandardUpdaterController`, Settings UI (copy patterns from Boring Notch: `SparkleView`, check/download toggles).
3. **`updater/appcast.xml` + Pages** — `static.yml` deploy `updater/` only.
4. **`release.yml`** — manual dispatch, admin gate, `generate_appcast`, appcast commit, GitHub Release DMG.
5. **Build number policy** — script to stamp `CFBundleVersion` from appcast max + 1 (avoid reusing build `1`).
6. **Release notes** — HTML embedded in appcast (security-sensitive changes called out explicitly).
7. **Compatibility gate** — release prep fails if `station-wire` version not bumped when `station/` or `agent/` protocol surfaces changed (CI diff rule).

## Phase 0: Signing & notarization foundation

**Outcome:** CI and local scripts can produce a **notarized** `TradeAutopsy Station.app` and DMG.

- [ ] Apple Developer Program team ID; App ID `in.tradeautopsy.station`.
- [ ] Hardened Runtime entitlements (minimal first): network client, Keychain, Apple Events if needed, **no broad sandbox** until entitlements design is ADR’d (kill switch / DNS helper may need special casing).
- [ ] Store **Developer ID cert** + **notary credentials** in GitHub Actions secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `TEAM_ID`).
- [ ] Extend `pack-station-dmg.sh` or add `scripts/sign-and-notarize-station.sh` (sign app → dmg → `notarytool submit` → staple).
- [ ] Document **internal unsigned** lane: keep `--ad-hoc` path for contributors without certs.

**Verify:** Download DMG on a clean Mac; double-click install without “Move to Trash” (after first Open).

## Phase 1: Sparkle client integration (tracer bullet)

**Outcome:** Dev-signed build checks feed manually; “Check for Updates” appears in Settings/About.

- [ ] Add Sparkle via Swift Package Manager (pin version in `Package.resolved`; mirror `fetch-generate-appcast` action).
- [ ] `Info.plist`: `SUFeedURL`, `SUPublicEDKey`, `SUEnableDownloaderService`, `SUEnableInstallerLauncherService` (enable XPC when sandbox added).
- [ ] App delegate / SwiftUI host: `SPUStandardUpdaterController(startingUpdater: true)`.
- [ ] UI: “Software updates” section — automatic check, automatic download, manual **Check for Updates** (strings already implied by Boring Notch pattern).
- [ ] If/when app is sandboxed: Mach lookup exceptions `in.tradeautopsy.station-spks` / `-spki` (Sparkle convention).

**Verify:** Point `SUFeedURL` at a **test appcast** with a higher build number; Sparkle offers update; install replaces app and **agent binary still present** at `Contents/MacOS/tradeautopsy-agent`.

## Phase 2: Feed hosting (GitHub Pages)

**Outcome:** Public URL serves signed appcast; no secrets in repo.

- [ ] Add `updater/appcast.xml` (seed with current `0.1.0` / build `1` or first signed release).
- [ ] Add `.github/workflows/static.yml` — on push to `main`, deploy **`updater/`** to GitHub Pages (project site: `tradeautopsy-station` repo).
- [ ] Optional: CNAME `updates.tradeautopsy.in` when DNS ready.
- [ ] `NSAppTransportSecurity`: prefer **HTTPS-only** for feed; avoid `NSAllowsArbitraryLoads` unless a documented exception.

**Verify:** `curl` appcast URL matches committed `updater/appcast.xml` after merge.

## Phase 3: Release pipeline (`release.yml`)

**Outcome:** Admin runs workflow → draft/published GitHub Release + updated appcast + notarized DMG.

Adapt Boring Notch **`release.yml`** structure:

| Job | Station-specific notes |
|-----|-------------------------|
| Preparation | Admin-only; `extract_version.py` (stable vs `-rc`); compute next **`CFBundleVersion`** from appcast + optional dev appcast |
| Build | `cargo build --release tradeautopsy-agent`; `xcodegen`; **Release** arm64 (extend to universal later if needed) |
| Sign & notarize | Before DMG |
| `generate_appcast` | `--download-url-prefix` → `.../releases/download/v${VERSION}/`; `--embed-release-notes`; beta → `--channel beta` |
| Publish stable | Replace `updater/appcast.xml` |
| Publish beta | `merge_appcast_channel.py` into main appcast |
| Wire gate | Fail if protocol-breaking diff without `station-wire/v1.json` version bump |
| Git tag | `v0.2.0` aligned with semver input |

Secrets: `PRIVATE_SPARKLE_KEY`, Apple notary secrets, optional GitHub App token for protected `main` commits.

**Verify:** Dry-run on internal beta channel with build `2`; install over `1`; app launches; agent listens on `9137`.

## Phase 4: Beta / dogfood channel

**Outcome:** RC builds do not auto-offer to stable users.

- [ ] Document: stable items have **no** `<sparkle:channel>`; beta items use `beta`.
- [ ] Optional: Settings toggle “Receive beta updates” → Sparkle 2 channel API (only if product wants in-app opt-in; otherwise separate beta appcast URL for dogfood builds).
- [ ] `updater/appcast-dev.xml` + **nightly** job (optional, later) — never point production `SUFeedURL` at dev.

**Verify:** Stable install on build `N` ignores beta item `N+1` on beta channel.

## Phase 5: Homebrew cask (optional)

**Outcome:** `brew install --cask tradeautopsy-station` tracks same DMG as Sparkle.

- [ ] Tap repo `homebrew-tradeautopsy` or org tap; cask `tradeautopsy-station` + `tradeautopsy-station@rc`.
- [ ] `auto_updates true`; `livecheck` `:github_latest` or appcast URL.
- [ ] `postflight` strip quarantine; `uninstall quit: in.tradeautopsy.station`.
- [ ] **`release.yml` job `upgrade-brew`** — regenerate cask SHA256 from release DMG (same as Boring Notch).

**Verify:** `brew upgrade` after a release produces same version as Sparkle check.

## Phase 6: Operational & security policy

**Outcome:** Updates are auditable and safe for a kill-switch product.

- [ ] **Release notes** must mention: agent changes, DNS/sudo helper changes, new TCC permissions, broker adapter ABI bumps.
- [ ] **Rollback**: keep previous DMG on GitHub Releases; Sparkle does not auto-downgrade — document manual reinstall.
- [ ] **Key rotation**: procedure to ship new `SUPublicEDKey` + dual-sign period (Sparkle docs).
- [ ] **Telemetry**: none required for updater; optional log line locally when update applied (version + build).
- [ ] **Compliance**: pin `station_version` in release artifact metadata (plist custom key or embedded `station-wire` JSON in bundle Resources).

## Phase 7: Deprecate ad-hoc distribution paths (soft)

- [ ] README: production users → Sparkle or Homebrew; CI artifact remains **7-day dev** only.
- [ ] Rename DMG to consistent **`TradeAutopsy-Station.dmg`** on releases (git sha suffix only for CI artifacts).

## Suggested file map (after implementation)

```
tradeautopsy-station/
  updater/
    appcast.xml              # GitHub Pages root → /appcast.xml
    appcast-dev.xml          # optional nightly
  .github/
    workflows/
      release.yml            # ship
      static.yml             # Pages deploy updater/
    actions/
      fetch-generate-appcast/
    scripts/
      extract_version.py
      merge_appcast_channel.py
      stamp_version.py       # CFBundleVersion + short version
  station/StationApp/Info.plist   # SUFeedURL, SUPublicEDKey
  scripts/pack-station-dmg.sh     # notarized path
```

## Open decisions (founder)

1. **Hosting brand:** GitHub Pages only vs `updates.tradeautopsy.in`.
2. **Sandbox:** Sparkle with full sandbox (like Boring Notch) vs Hardened Runtime only first — affects entitlements and DNS helper story.
3. **Universal binary:** arm64-only vs arm64+x86_64 for Sparkle delta updates (deltas optional in v1).
4. **In-app beta toggle** vs separate dogfood feed URL.
5. **When to cut first signed release:** before or after a specific P9 dogfood milestone.

## Success criteria

- Stable user on release **N** gets **N+1** via “Check for Updates” without manual DMG.
- Appcast signature verifies with embedded public key; tampered feed rejected.
- Post-update: agent binary version matches app build; Notch hotkey works; loopback `9137` healthy.
- Beta channel does not upgrade stable users unless opted in.
- Wire protocol version in release notes matches `station-wire/v1.json`.

## Implementation order (recommended)

```
Phase 0 (signing) → Phase 1 (Sparkle in app) → Phase 2 (Pages) → Phase 3 (release.yml)
→ Phase 4 (beta) → Phase 5 (Homebrew, optional) → Phase 6 (policy) → Phase 7 (docs)
```

Phase 1 can use a **staging appcast** before Phase 3 is complete; do not point production `SUFeedURL` at staging until signing is real (Phase 0).
