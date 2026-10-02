# Station CI — what runs when

| Workflow | Fires on | Runner | Does |
| --- | --- | --- | --- |
| `ci.yml` | every PR + push to `main` (skips docs/plans/md-only diffs) | `ubuntu-latest` + `macos-15` | Linux: builds every `agent/ubi-*-adapter` for `wasm32-wasip2`. macOS: `cargo build --lib` + `cargo test --lib` (`RUSTFLAGS=-D unused`) |
| `macos-app.yml` | PR/push only when `station/**` or `notch/**` change + dispatch | `macos-15` | `swift build/test` for `notch/` and `station/`, xcodegen + `xcodebuild` Debug. Zip/DMG + artifact upload only off-PR |
| `nightly.yml` | cron `17 3 * * *` + dispatch | `macos-15` | `cargo test --lib`, then `scripts/station-path-stress.sh` if present. Dispatch input `live_soak=true` additionally runs `scripts/station-live-stress-soak.sh` — only when the `STATION_LIVE_SOAK_ENV_B64` secret exists |
| `release.yml` | dispatch | `macos-15` | signed/notarized DMG, GitHub release, appcast commit |
| `deploy-updater-vercel.yml` | dispatch | `ubuntu-latest` | publishes `updater/appcast.xml` to the Vercel feed |
| `static.yml` | dispatch | `ubuntu-latest` | optional Pages backup of the appcast |
| `agent-dispatch.yml` | dispatch | `ubuntu-latest` | hands `ready-for-agent` issues to Cursor Cloud Agents |

## Notes

- **Why macOS for agent tests:** `keyring` (`apple-native`) and `notify`
  (`macos_kqueue`) are unconditional deps, so the crate does not compile on
  Linux. Moving agent unit tests to `ubuntu-latest` needs a `cfg`-gating pass —
  tracked as a follow-up, not done here.
- **Integration tests:** `agent/tests/*` are not in the PR gate; run them before
  release (`cargo test` in `agent/`). `tests/kill_latch_lifetime.rs` currently
  fails to compile on `main` — pre-existing, unrelated to these workflows.
- **Stress harnesses** (`station-path-stress.sh`, `station-live-stress-soak.sh`)
  land via PRs; `nightly.yml` skips them with a notice until they exist.
- **Secrets:** workflows reference secrets only via `${{ secrets.* }}`; nothing
  sensitive is committed. Live soak material arrives as a base64 blob decoded to
  `~/.tradeautopsy/station.env` at runtime.
