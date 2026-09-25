# Runbook: `updates.tradeautopsy.in` (GoDaddy DNS + GitHub Pages)

Backup only. The live feed is the Vercel project **updater**. DNS for `updates` must point at Vercel, not at `fexevil.github.io`, while that project serves the feed. See [updates-domain-vercel.md](./updates-domain-vercel.md). Do not put both targets on the same host.

Make the Sparkle feed live at `https://updates.tradeautopsy.in/appcast.xml`.

That URL is already `SUFeedURL` in `station/StationApp/Info.plist`. Do not change it. The github.io address below is only a manual fallback until this hostname answers.

| | |
| --- | --- |
| App feed (keep this) | `https://updates.tradeautopsy.in/appcast.xml` |
| Fallback until DNS works | `https://fexevil.github.io/tradeautopsy-station/appcast.xml` |
| DNS record | `updates` CNAME `fexevil.github.io` |
| Pages artifact | `updater/` via `.github/workflows/static.yml` |
| Custom-domain file | `updater/CNAME` (`updates.tradeautopsy.in`) |

Subdomain only. Do not change the apex (`@`) records for `tradeautopsy.in` — no new A/AAAA on `@`, no nameserver change, no redirect of the apex to GitHub.

## Before you edit DNS

1. Sign in to the GoDaddy account that owns `tradeautopsy.in`.
2. Confirm this zone is the one the world uses:

   ```bash
   dig +short NS tradeautopsy.in
   ```

   Live GoDaddy DNS looks like `ns**.domaincontrol.com`. If the nameservers are somewhere else (Cloudflare, Route 53, and so on), a record saved in GoDaddy will not publish. Stop and edit the provider that actually hosts the zone.
3. The Pages site is not on GitHub until commit `ae18bde` (*Add Sparkle update pipeline and signed-release infrastructure.*) is on `origin/main`. That commit adds `updater/` and `static.yml`. A later successful run of **Deploy static content to Pages** on `main` is enough. `git push` does not upload uncommitted files. Do not commit unrelated `station/StationApp` broker or Today edits in order to publish the feed.

## GoDaddy

Click-path: **My Products → tradeautopsy.in → DNS → Add → CNAME**.

Same screen from the domain list: [GoDaddy DNS for tradeautopsy.in](https://dcc.godaddy.com/control/dnsmanagement?domainName=tradeautopsy.in) → **DNS Records** → **Add** (sometimes labeled **Add New Record**).

| Field | Value |
| --- | --- |
| Type | CNAME |
| Name / Host | `updates` |
| Value / Points to | `fexevil.github.io` |
| TTL | `600` seconds, or the default **1 Hour** |

Save. The new row should read host `updates`, type CNAME, data `fexevil.github.io`.

### GoDaddy quirks

- **Name is the label only.** Type `updates`, not `updates.tradeautopsy.in`. The UI appends `.tradeautopsy.in`. The full name in that box becomes `updates.tradeautopsy.in.tradeautopsy.in`.
- **Target is the Pages user host, not the repo path.** `fexevil.github.io` is correct. `fexevil.github.io/tradeautopsy-station` and `https://fexevil.github.io/...` are not CNAME targets. Once the custom domain is attached, the feed is at the **root** of that host (`/appcast.xml`), not under `/tradeautopsy-station/`.
- **No second record on `updates`.** Delete any existing A, AAAA, or CNAME whose host is `updates` before saving this one. GoDaddy will refuse a CNAME, or will keep serving the A record, while another record exists on that name.
- **Forwarding is not a CNAME.** Do not add domain or subdomain forwarding for `updates`. Forwarding writes A records and blocks the CNAME. If the UI mentions CNAME flattening, or refuses the CNAME because records already exist, remove those records first, then add the CNAME again.
- **Trailing dot.** Prefer `fexevil.github.io` with no trailing dot. GoDaddy often rejects the dot; DNS still means the same name if a dot is accepted.
- **TTL.** `600` matches a 10-minute cache. **1 Hour** is fine if the form has no custom TTL.

## GitHub (FExEVIL/tradeautopsy-station)

Repo settings: [Pages](https://github.com/FExEVIL/tradeautopsy-station/settings/pages). You need admin (or “manage GitHub Pages”) on the repo.

1. **Build and deployment → Source** = **GitHub Actions**. Not “Deploy from a branch”. The workflow uploads `updater/` only.
2. **Custom domain** = `updates.tradeautopsy.in` → **Save**. This must match `updater/CNAME` exactly (one line, no `https://`, no path).
3. Wait for the DNS check on that page. **DNS check unsuccessful** means GitHub cannot see the CNAME yet. Fix the GoDaddy row, or wait for propagation, then use **Check again** if it is shown.
4. When **Enforce HTTPS** is offered (the box is not greyed out), turn it on. GitHub may take up to an hour after DNS is correct to issue the certificate, and up to 24 hours before the checkbox appears. Leave the box alone while it is disabled. Do not serve the feed over plain HTTP.
5. **Actions → Deploy static content to Pages.** Confirm a green run on `main` for `ae18bde` or any later commit. Push `main` first if that commit is still only on your machine. Re-run the workflow with **Run workflow** after the custom domain is saved if the latest run is older than the `updater/` contents you expect.

### If the custom domain still fails

- **Settings → Environments:** environment `github-pages` exists. `static.yml` declares it; the first successful deploy creates it.
- The **deploy** job in `static.yml` succeeded. A failed `upload-pages-artifact` / `deploy-pages` step means the feed was never published.
- `updater/CNAME` is in the artifact (the workflow path is `updater/`, so `CNAME` lands at the site root). A CNAME only in the Git UI, missing from the artifact, gets dropped on the next Actions deploy.
- HTTPS never unlocks, and the apex already has CAA records: at least one CAA value must be `letsencrypt.org`, or GitHub cannot issue the certificate. Do not add a CAA record if the zone has none.
- Certificate stuck after DNS is clearly right: on the Pages screen, remove `updates.tradeautopsy.in`, save, add the same domain again, save. That retriggers issuance. It does not require an apex change.

## Propagation

With TTL 600, resolvers usually pick up the CNAME in **5–60 minutes**. A resolver that cached “name does not exist” can stay stale until that negative cache expires (often inside the same window, sometimes closer to an hour). GitHub’s DNS check and the Enforce HTTPS checkbox can lag the public CNAME by longer, up to 24 hours.

Bypass a stale laptop resolver:

```bash
dig +short CNAME updates.tradeautopsy.in @8.8.8.8
```

Expect `fexevil.github.io.` (trailing dot is normal here).

## Verify

From the repo root:

```bash
./scripts/verify-updates-feed.sh
```

Or the same checks by hand:

```bash
dig +short CNAME updates.tradeautopsy.in
curl -sI https://updates.tradeautopsy.in/appcast.xml
curl -sS https://updates.tradeautopsy.in/appcast.xml | head -5
```

Pass looks like:

- CNAME target `fexevil.github.io` (a trailing dot is fine)
- Final HTTP status **200**
- `Content-Type` contains `xml` or `html`
- The first lines are a Sparkle RSS channel (`<rss`, `sparkle`, `<channel>`)

Until that passes, check whether Pages has published the artifact at all:

```bash
curl -sI https://fexevil.github.io/tradeautopsy-station/appcast.xml
```

HTTP 200 there means the workflow has deployed and only the custom domain is left. HTTP 404 with a “Site not found” title means GitHub Pages has not published this repo yet — push `main` through `ae18bde` and wait for a green **Deploy static content to Pages** run before debugging DNS. That github.io URL is for curl and a browser. Leave `SUFeedURL` on `https://updates.tradeautopsy.in/appcast.xml`.
