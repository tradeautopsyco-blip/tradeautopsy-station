# TradeAutopsy Station Sparkle feed

This directory is deployed to **GitHub Pages** on every push to `main` (see `.github/workflows/static.yml`).

## Feed URL

After Pages is enabled for this repository, clients should use:

| Hosting | URL |
|---------|-----|
| Custom domain (recommended) | `https://updates.tradeautopsy.in/appcast.xml` |
| GitHub Pages default | `https://fexevil.github.io/tradeautopsy-station/appcast.xml` |

`SUFeedURL` in `station/StationApp/Info.plist` stays `https://updates.tradeautopsy.in/appcast.xml`. The github.io URL is a manual fallback until that host answers. Do not retarget the plist to github.io.

The `CNAME` file pins the custom hostname to `updates.tradeautopsy.in`.

## DNS (CNAME)

At GoDaddy for `tradeautopsy.in`, add a **subdomain** record only (do not edit apex `@`):

```
updates.tradeautopsy.in  CNAME  fexevil.github.io.
```

`fexevil.github.io` is the GitHub Pages user host. It is not `fexevil.github.io/tradeautopsy-station`. Click-path, GitHub Pages settings, and HTTPS: [docs/runbooks/updates-domain-godaddy.md](../docs/runbooks/updates-domain-godaddy.md).

In the repo **Settings → Pages**, set source to **GitHub Actions** (the `static.yml` workflow uploads `updater/`) and custom domain `updates.tradeautopsy.in`.

## Verification

DNS usually shows up in 5–60 minutes after the CNAME is saved. Enforce HTTPS on GitHub can take longer (up to 24 hours before the checkbox is offered).

```bash
./scripts/verify-updates-feed.sh
```

The script checks, in order:

1. `dig +short CNAME updates.tradeautopsy.in` → `fexevil.github.io`
2. `curl -sI https://updates.tradeautopsy.in/appcast.xml` → HTTP 200 and a `Content-Type` containing `xml` or `html`
3. The first five lines of the body are a Sparkle RSS channel
4. SHA-256 of the live body matches `updater/appcast.xml` (so Pages is serving this commit’s feed)

Any failed check exits non-zero. The github.io URL is a manual fallback only, not the plist URL. It returns GitHub’s “Site not found” page until **Deploy static content to Pages** has succeeded on `origin/main`:

```bash
curl -sI https://fexevil.github.io/tradeautopsy-station/appcast.xml
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
