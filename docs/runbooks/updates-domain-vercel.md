# Runbook: `updates.tradeautopsy.in` on Vercel

Hosts the Sparkle feed from `updater/` (static `appcast.xml`). Matches `SUFeedURL` in `station/StationApp/Info.plist`.

| | |
| --- | --- |
| Production URL (until custom domain) | `https://updater-omega.vercel.app/appcast.xml` |
| Custom domain (target) | `https://updates.tradeautopsy.in/appcast.xml` |
| Vercel project | [tradeautospy/updater](https://vercel.com/tradeautospy/updater) |
| Deploy | `cd updater && vercel deploy --prod` or Git connect after `vercel git connect` |

## Vercel domain

`updates.tradeautopsy.in` is attached to project **updater** (`prj_7bsoGXrasUUufotLOpLQugBwpsKa`). It is **not verified** until the TXT below is in DNS. `vercel domains add` is the wrong command here: it tries to take registrar ownership and returns `domain_not_owned`. The apex stays on GoDaddy and on the `tradeautopsy` project.

## GoDaddy DNS (Vercel)

**My Products → tradeautopsy.in → DNS → Add.** Two records. Do not edit apex `@`, and do not delete the existing `_vercel` TXT rows (those verify `tradeautopsy.in` and `www`).

| Type | Name | Value | TTL |
| --- | --- | --- | --- |
| CNAME | `updates` | `f636bc2918b40e49.vercel-dns-017.com` | 600 or 1 hour |
| TXT | `_vercel` | `vc-domain-verify=updates.tradeautopsy.in,be49cb4a727fc3804b2f` | 600 or 1 hour |

`cname.vercel-dns.com` is an accepted alternate CNAME target. Prefer the project-specific host above; that is the record Vercel ranks first for this domain.

Remove any A, AAAA, or other CNAME on the host `updates` before saving. Do not point `updates` at `fexevil.github.io` while this project serves the feed. One DNS target only.

Name is the label only (`updates`, `_vercel`). GoDaddy appends `.tradeautopsy.in`.

After both records propagate, Vercel → **updater** → **Settings → Domains** should show `updates.tradeautopsy.in` verified, with HTTPS. If it stays pending, use **Refresh** on that domain. Verification checks the TXT, then issues the certificate.

## Verify

```bash
dig +short CNAME updates.tradeautopsy.in @8.8.8.8
# expect: f636bc2918b40e49.vercel-dns-017.com.
# also valid: cname.vercel-dns.com.

dig +short TXT _vercel.tradeautopsy.in @8.8.8.8
# must include vc-domain-verify=updates.tradeautopsy.in,be49cb4a727fc3804b2f
# and still include the existing tradeautopsy.in and www values

./scripts/verify-updates-feed.sh
```

## Redeploy after appcast changes

When `updater/appcast.xml` changes on `main`, run:

```bash
cd updater && vercel deploy --prod --yes
```

`updater/CNAME` is not in this tree. That file would make GitHub Pages claim `updates.tradeautopsy.in` on the next Pages deploy. Pages can still serve `https://fexevil.github.io/tradeautopsy-station/appcast.xml` after a successful `static.yml` run. Do not set that same custom domain on Pages.

Git auto-deploy is not linked. Connecting the repo only works if the Vercel project's root directory is `updater`. Root is `.` today because deploys are `cd updater && vercel deploy`. Leave it that way until the GitHub app can see `FExEVIL/tradeautopsy-station`; then set the root directory and connect in the dashboard. A git deploy with root `.` would publish the whole station repo and replace this feed.
