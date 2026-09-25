# Runbook: `updates.tradeautopsy.in` on Vercel

Hosts the Sparkle feed from `updater/` (static `appcast.xml`). Matches `SUFeedURL` in `station/StationApp/Info.plist`.

| | |
| --- | --- |
| Production URL (until custom domain) | `https://updater-omega.vercel.app/appcast.xml` |
| Custom domain (target) | `https://updates.tradeautopsy.in/appcast.xml` |
| Vercel project | [tradeautospy/updater](https://vercel.com/tradeautospy/updater) |
| Deploy | `cd updater && vercel deploy --prod` or Git connect after `vercel git connect` |

## GoDaddy DNS (Vercel)

**My Products → tradeautopsy.in → DNS → Add → CNAME**

| Field | Value |
| --- | --- |
| Type | CNAME |
| Name | `updates` |
| Value | `cname.vercel-dns.com` |
| TTL | 600 or 1 hour |

Remove any existing A/AAAA/CNAME on `updates` first. Do not use `fexevil.github.io` if Vercel is the host (that target is for GitHub Pages only).

After saving, in Vercel → **updater** → **Settings → Domains**, confirm `updates.tradeautopsy.in` is verified and HTTPS is active.

## Verify

```bash
dig +short CNAME updates.tradeautopsy.in @8.8.8.8
# expect: cname.vercel-dns.com.

./scripts/verify-updates-feed.sh
```

## Redeploy after appcast changes

When `updater/appcast.xml` changes on `main`, run:

```bash
cd updater && vercel deploy --prod --yes
```

Optional: connect the GitHub repo in the Vercel dashboard so pushes redeploy automatically (CLI `vercel git connect` if the GitHub app has access to `FExEVIL/tradeautopsy-station`).
