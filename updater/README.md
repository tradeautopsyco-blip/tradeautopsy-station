# TradeAutopsy Station Sparkle feed

This directory is deployed to **GitHub Pages** on every push to `main` (see `.github/workflows/static.yml`).

## Feed URL

After Pages is enabled for this repository, clients should use:

| Hosting | URL |
|---------|-----|
| Custom domain (recommended) | `https://updates.tradeautopsy.in/appcast.xml` |
| GitHub Pages default | `https://fexevil.github.io/tradeautopsy-station/appcast.xml` |

Set `SUFeedURL` in the app’s `Info.plist` to the HTTPS URL you ship (custom domain once DNS is live).

The `CNAME` file pins the custom hostname to `updates.tradeautopsy.in`.

## DNS (CNAME)

At your DNS provider for `tradeautopsy.in`, add:

```
updates.tradeautopsy.in  CNAME  fexevil.github.io.
```

(Use `fexevil.github.io` as the GitHub Pages target for a **project** site on `FExEVIL/tradeautopsy-station`.)

In the repo **Settings → Pages**, set source to **GitHub Actions** (the `static.yml` workflow uploads `updater/`).

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
