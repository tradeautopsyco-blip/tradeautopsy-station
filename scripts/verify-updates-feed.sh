#!/usr/bin/env bash
# Confirm updates.tradeautopsy.in CNAMEs to GitHub Pages and serves the Sparkle appcast.
# DNS often takes 5–60 minutes after the GoDaddy record is saved.
# Exit 0 only when every check passes. Does not change SUFeedURL.
set -euo pipefail

HOST="updates.tradeautopsy.in"
EXPECTED_CNAME="fexevil.github.io"
FEED_URL="https://${HOST}/appcast.xml"
FALLBACK_URL="https://fexevil.github.io/tradeautopsy-station/appcast.xml"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCAL_APPCAST="${ROOT}/updater/appcast.xml"

fail() {
  echo "FAIL: $*" >&2
  echo "Fallback until DNS works (do not put this in SUFeedURL): ${FALLBACK_URL}" >&2
  echo "Runbook: docs/runbooks/updates-domain-godaddy.md" >&2
  exit 1
}

digest() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    fail "Need shasum or sha256sum to compare the feed with updater/appcast.xml."
  fi
}

if ! command -v dig >/dev/null 2>&1; then
  fail "dig is not installed."
fi
if ! command -v curl >/dev/null 2>&1; then
  fail "curl is not installed."
fi

if ! raw_cname="$(dig +short CNAME "$HOST")"; then
  fail "dig failed for ${HOST}."
fi
cname="$(printf '%s\n' "$raw_cname" | awk 'NR==1{print tolower($0)}' | sed 's/\.$//' | tr -d '[:space:]')"
if [[ -z "$cname" ]]; then
  ns="$(dig +short NS tradeautopsy.in || true)"
  ns="$(printf '%s' "$ns" | tr '\n' ' ')"
  fail "No CNAME for ${HOST}. In GoDaddy add host updates → ${EXPECTED_CNAME}, and delete any A/AAAA on that host. Typical propagation is 5–60 minutes. Nameservers: ${ns:-unknown} Try: dig +short CNAME ${HOST} @8.8.8.8"
fi
if [[ "$cname" != "$EXPECTED_CNAME" ]]; then
  fail "CNAME for ${HOST} is ${cname}; expected ${EXPECTED_CNAME}. Point the updates record at the GitHub Pages user host, not the repository path."
fi
echo "OK  CNAME ${HOST} → ${cname}"

header_file="$(mktemp)"
body_file="$(mktemp)"
cleanup() {
  rm -f "$header_file" "$body_file"
}
trap cleanup EXIT

if ! curl -sS -I -L --max-redirs 5 --max-time 30 "$FEED_URL" >"$header_file"; then
  fail "Header request failed for ${FEED_URL}. If this is a certificate error, wait until GitHub Pages offers Enforce HTTPS (up to 24 hours after DNS is correct)."
fi

status="$(tr -d '\r' <"$header_file" | awk '/^HTTP/{code=$2} END{print code}')"
if [[ "$status" != "200" ]]; then
  fail "Expected HTTP 200 from ${FEED_URL}, got ${status:-none}. 404 after a correct CNAME means the Pages custom domain is not saved, or Deploy static content to Pages has not succeeded on main."
fi

ctype="$(tr -d '\r' <"$header_file" | grep -i '^content-type:' | tail -n 1 || true)"
if [[ -z "$ctype" ]]; then
  fail "No Content-Type on ${FEED_URL}."
fi
ctype_lower="$(printf '%s' "$ctype" | tr '[:upper:]' '[:lower:]')"
case "$ctype_lower" in
  *xml*|*html*) ;;
  *) fail "Content-Type must contain xml or html, got: ${ctype}" ;;
esac
echo "OK  HTTP ${status} ${ctype}"

if ! curl -sS -L --max-redirs 5 --max-time 30 -o "$body_file" "$FEED_URL"; then
  fail "Body request failed for ${FEED_URL}."
fi

head5="$(head -n 5 "$body_file")"
echo "---- feed head ----"
printf '%s\n' "$head5"
echo "---- end head ----"

grep -q '<rss' <<<"$head5" || fail "First 5 lines are not an RSS document (missing <rss)."
grep -q 'sparkle' <<<"$head5" || fail "First 5 lines are missing the Sparkle namespace."
grep -q '<channel>' <<<"$head5" || fail "First 5 lines are missing <channel>."
echo "OK  Sparkle RSS channel"

if [[ ! -f "$LOCAL_APPCAST" ]]; then
  fail "Local appcast not found at ${LOCAL_APPCAST}."
fi

local_hash="$(digest "$LOCAL_APPCAST")"
remote_hash="$(digest "$body_file")"
if [[ "$local_hash" != "$remote_hash" ]]; then
  if [[ "$(head -c 200 "$LOCAL_APPCAST")" == "$(head -c 200 "$body_file")" ]]; then
    prefix_note="First 200 bytes match; bytes after that differ."
  else
    prefix_note="First 200 bytes differ."
  fi
  fail "Live feed does not match updater/appcast.xml. ${prefix_note} local sha256 ${local_hash}; remote sha256 ${remote_hash}. Redeploy Pages from main (workflow: Deploy static content to Pages)."
fi
echo "OK  sha256 matches updater/appcast.xml (${local_hash})"
echo "All checks passed."
