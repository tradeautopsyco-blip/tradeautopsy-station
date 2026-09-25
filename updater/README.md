# TradeAutopsy Station Sparkle feed

This directory is served as the Sparkle feed on **Vercel** (primary):

- Production: `https://updater-omega.vercel.app/appcast.xml`
- Custom domain (after GoDaddy CNAME): `https://updates.tradeautopsy.in/appcast.xml`

Deploy: `cd updater && vercel deploy --prod --yes` — see [docs/runbooks/updates-domain-vercel.md](../docs/runbooks/updates-domain-vercel.md).

**GitHub Pages** (optional backup): `.github/workflows/static.yml` on push to `main`.

## Feed URL

| Hosting | URL |
|---------|-----|
| Custom domain (target, already `SUFeedURL`) | `https://updates.tradeautopsy.in/appcast.xml` |
| Vercel production (live now) | `https://updater-omega.vercel.app/appcast.xml` |

`SUFeedURL` in `station/StationApp/Info.plist` stays on the custom domain. Do not retarget shipped apps to `updater-omega.vercel.app`. That host is the manual fallback until GoDaddy verification finishes.

## DNS (CNAME)

At GoDaddy for `tradeautopsy.in`, subdomain only (do not edit apex `@`):

```
updates.tradeautopsy.in  CNAME  f636bc2918b40e49.vercel-dns-017.com.
_vercel.tradeautopsy.in  TXT    vc-domain-verify=updates.tradeautopsy.in,be49cb4a727fc3804b2f
```

Keep the existing `_vercel` TXT records for `tradeautopsy.in` and `www`. Exact click-path: [docs/runbooks/updates-domain-vercel.md](../docs/runbooks/updates-domain-vercel.md).

Do not also CNAME `updates` to `fexevil.github.io`. GitHub Pages is an optional backup at the github.io URL only, without this custom domain.

## Verification

DNS usually shows up in 5–60 minutes after the CNAME and TXT are saved.

```bash
./scripts/verify-updates-feed.sh
```

The script checks, in order:

1. `dig +short CNAME updates.tradeautopsy.in` → a Vercel DNS host (`f636bc2918b40e49.vercel-dns-017.com` or `cname.vercel-dns.com`)
2. `curl -sI https://updates.tradeautopsy.in/appcast.xml` → HTTP 200 and a `Content-Type` containing `xml` or `html`
3. The first five lines of the body are a Sparkle RSS channel
4. SHA-256 of the live body matches `updater/appcast.xml`

Any failed check exits non-zero. Until the custom domain answers, the production alias already serves this file:

```bash
curl -sI https://updater-omega.vercel.app/appcast.xml
```

## Sparkle signing keys

Sparkle uses an **Ed25519** key pair:

1. Generate once (local Mac with Sparkle tools installed):
   ```bash
   ./Sparkle/bin/generate_keys
   ```
   Or use `generate_appcast --generate-key` from a Sparkle release tarball.

2. Put the **public** key in the app as `SUPublicEDKey` in `Info.plist` (must match the private key in CI).

   The repo currently ships public key `VcUjJxkUER6Nut2xnSdrA4KpJ8Dg3av4osZqOE/ayKw=` (generated via Sparkle `generate_keys`). Export the matching **private** Ed25519 key into GitHub Actions secret `PRIVATE_SPARKLE_KEY` before running `release.yml` (Sparkle Keychain export or `generate_appcast` tooling — never commit the private key).

3. Store the **private** key in GitHub Actions as repository secret `PRIVATE_SPARKLE_KEY` (PEM text). The release workflow pipes it to `generate_appcast --ed-key-file -`.

4. Never commit the private key. Rotate only with a coordinated app + appcast migration (see [Sparkle documentation](https://sparkle-project.org/documentation/)).

## Appcast maintenance

- **Stable** releases replace `updater/appcast.xml` via the `release.yml` workflow.
- **Beta / RC** releases (`*-rc.*`, semver pre-release) merge a `beta` channel item with `.github/scripts/merge_appcast_channel.py`.
- The placeholder `sparkle:edSignature` in the seed appcast is invalid on purpose; the first signed release overwrites it with a real signature.

## Release artifacts

DMGs are published to [GitHub Releases](https://github.com/FExEVIL/tradeautopsy-station/releases) as `TradeAutopsy-Station.dmg`. The appcast enclosure URL must match:

`https://github.com/FExEVIL/tradeautopsy-station/releases/download/v{VERSION}/TradeAutopsy-Station.dmg`
