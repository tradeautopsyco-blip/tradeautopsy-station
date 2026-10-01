#!/usr/bin/env python3
"""Paper-only Station stress soak. Never logs secrets or places broker orders.

Ramps concurrency per path until the first failure or the configured cap.
See plans/STATION-LIVE-STRESS-SOAK.md.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
import os
import plistlib
import re
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Callable, Optional

READ_LEVELS = (1, 4, 16, 32, 64)
WRITE_LEVELS = (1, 2, 4, 8, 16, 32)
TOTP_CARD_SECONDS = 12 * 60 * 60
APPCAST_URL = "https://updates.tradeautopsy.in/appcast.xml"
DEBUG_APP_GLOBS = (
    os.path.expanduser(
        "~/Library/Developer/Xcode/DerivedData/"
        "TradeAutopsy_Station-ccdwmgacytrtxkdikkdnlwopgfkd/"
        "Build/Products/Debug/TradeAutopsy Station.app"
    ),
)
RELEASE_APP = "/Applications/TradeAutopsy Station.app"
STATION_DEFAULTS_DOMAIN = "in.tradeautopsy.station"
METADATA_PREFIX = "station.broker.metadata."

SENSITIVE_KEY = re.compile(
    r"token|secret|email|authorization|signature|password|api[_-]?key|"
    r"totp|mpin|cookie|bearer|pnl|balance|funds_balance|\bnet\b",
    re.I,
)


@dataclass
class Probe:
    ok: bool
    note: str
    elapsed: float = 0.0
    http: int = 0


@dataclass
class PathResult:
    path: str
    passed_n: int
    ok_requests: int
    first_failure: str
    notes: str = ""
    waves: list[dict[str, Any]] = field(default_factory=list)


def load_journal_lib(root: str) -> Any:
    path = os.path.join(root, "scripts", "notch-live-journal-lib.py")
    spec = importlib.util.spec_from_file_location("notch_live_journal_lib", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    mod = importlib.util.module_from_spec(spec)
    # Python 3.14 dataclasses look up cls.__module__ during class body exec.
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def redact(obj: Any, depth: int = 0) -> Any:
    if depth > 6:
        return "[truncated]"
    if isinstance(obj, dict):
        out: dict[str, Any] = {}
        for key, value in obj.items():
            if SENSITIVE_KEY.search(str(key)):
                out[str(key)] = "[redacted]"
            else:
                out[str(key)] = redact(value, depth + 1)
        return out
    if isinstance(obj, list):
        return [redact(item, depth + 1) for item in obj[:12]]
    if isinstance(obj, str) and len(obj) > 180:
        return obj[:48] + "…"
    return obj


def safe_excerpt(text: str, limit: int = 180) -> str:
    if not text:
        return ""
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        scrubbed = re.sub(r"[A-Za-z0-9_\-]{24,}", "[redacted]", text)
        return scrubbed[:limit]
    return json.dumps(redact(parsed), separators=(",", ":"))[:limit]


def p95(values: list[float]) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, math.ceil(0.95 * len(ordered)) - 1))
    return ordered[index]


def levels_up_to(table: tuple[int, ...], cap: int) -> list[int]:
    chosen = [n for n in table if n <= cap]
    if cap > 0 and (not chosen or chosen[-1] != cap) and cap not in chosen:
        if not chosen or cap > chosen[-1]:
            chosen.append(cap)
    return chosen or [1]


def read_server_base() -> str:
    path = os.path.expanduser("~/.tradeautopsy/station.env")
    try:
        text = open(path, encoding="utf-8").read()
    except OSError:
        return "(station.env unreadable)"
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("TRADEAUTOPSY_SERVER_BASE_URL="):
            return stripped.split("=", 1)[1].strip()
    return "(TRADEAUTOPSY_SERVER_BASE_URL unset)"


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


class Soak:
    def __init__(
        self,
        *,
        root: str,
        host: str,
        port: int,
        lib: Any,
        secret: str,
        max_read: int,
        max_write: int,
    ) -> None:
        self.root = root
        self.host = host
        self.port = port
        self.lib = lib
        self.secret = secret
        self.max_read = max_read
        self.max_write = max_write
        self.results: list[PathResult] = []
        self.orphans: list[str] = []
        self.orphan_lock = threading.Lock()
        self.log_lines: list[str] = []
        self.started = utc_now()

    def log(self, msg: str) -> None:
        line = f"{utc_now()} {msg}"
        self.log_lines.append(line)
        print(line, flush=True)

    def wire(self, method: str, path: str, body: Optional[bytes] = None, timeout: float = 25.0) -> Any:
        return self.lib.http_request(
            self.host,
            self.port,
            method,
            path,
            secret=self.secret,
            body=body,
            wire=True,
            timeout=timeout,
        )

    def open_get(self, path: str, timeout: float = 25.0) -> Any:
        return self.lib.http_request(
            self.host,
            self.port,
            "GET",
            path,
            secret=None,
            body=None,
            wire=False,
            timeout=timeout,
        )

    def agent_up(self) -> bool:
        return self.lib.port_open(self.host, self.port, timeout=0.6)

    def timed(self, fn: Callable[[], Probe]) -> Probe:
        t0 = time.perf_counter()
        try:
            probe = fn()
        except Exception as exc:  # noqa: BLE001 — soak must record the failure
            name = type(exc).__name__
            return Probe(False, f"{name}", elapsed=time.perf_counter() - t0)
        probe.elapsed = time.perf_counter() - t0
        return probe

    def ramp(
        self,
        path: str,
        levels: list[int],
        worker: Callable[[], Probe],
        *,
        notes: str = "",
        latency_break_s: float = 20.0,
    ) -> PathResult:
        passed = 0
        ok_requests = 0
        waves: list[dict[str, Any]] = []
        self.log(f"path {path} levels={levels}")
        if not self.agent_up() and path != "sparkle_feed":
            result = PathResult(path, 0, 0, "agent not listening", notes, waves)
            self.results.append(result)
            self.log(f"path {path} FAIL agent down")
            return result
        for level in levels:
            if path != "sparkle_feed" and not self.agent_up():
                failure = f"agent died before concurrency {level}"
                result = PathResult(path, passed, ok_requests, failure, notes, waves)
                self.results.append(result)
                self.log(f"path {path} FAIL {failure}")
                return result
            probes: list[Probe] = []
            with ThreadPoolExecutor(max_workers=level) as pool:
                futures = [pool.submit(self.timed, worker) for _ in range(level)]
                for fut in as_completed(futures):
                    probes.append(fut.result())
            failures = [p for p in probes if not p.ok]
            elapsed = [p.elapsed for p in probes]
            wave = {
                "concurrency": level,
                "ok": len(probes) - len(failures),
                "fail": len(failures),
                "p95_s": round(p95(elapsed), 3),
            }
            waves.append(wave)
            self.log(
                f"path {path} n={level} ok={wave['ok']} fail={wave['fail']} p95={wave['p95_s']}s"
            )
            if failures:
                note = failures[0].note
                result = PathResult(path, passed, ok_requests, note, notes, waves)
                self.results.append(result)
                self.log(f"path {path} first_failure={note}")
                return result
            slow = wave["p95_s"]
            if slow >= latency_break_s:
                note = f"latency p95 {slow}s at concurrency {level}"
                result = PathResult(path, passed, ok_requests, note, notes, waves)
                self.results.append(result)
                self.log(f"path {path} first_failure={note}")
                return result
            passed = level
            ok_requests += level
        cap = levels[-1] if levels else 0
        result = PathResult(
            path,
            passed,
            ok_requests,
            f"none (held through concurrency {cap})",
            notes,
            waves,
        )
        self.results.append(result)
        return result

    def add_static(self, path: str, passed_n: int, ok_requests: int, first_failure: str, notes: str = "") -> None:
        result = PathResult(path, passed_n, ok_requests, first_failure, notes)
        self.results.append(result)
        self.log(f"path {path} passed_n={passed_n} failure={first_failure}")

    # --- probes ---

    def probe_session(self) -> Probe:
        res = self.wire("GET", "/api/daemon/auth/station/session")
        body = res.json() if isinstance(res.json(), dict) else {}
        if res.status == 200 and body.get("signed_in") is True:
            return Probe(True, "signed_in", http=res.status)
        if res.status == 401 or body.get("error_class") == "SESSION_PROOF":
            return Probe(False, "SESSION_PROOF", http=res.status)
        if body.get("signed_in") is False:
            return Probe(False, "signed_out", http=res.status)
        return Probe(False, f"HTTP {res.status} {safe_excerpt(res.body_text)}", http=res.status)

    def probe_status(self) -> Probe:
        checks = [
            ("GET", "/api/daemon/health", True),
            ("GET", "/api/daemon/broker/sync-state", True),
            ("GET", "/api/daemon/today", True),
            ("GET", "/api/daemon/positions", True),
        ]
        for method, path, _wired in checks:
            res = self.wire(method, path)
            if res.status != 200:
                return Probe(False, f"{path} HTTP {res.status} {safe_excerpt(res.body_text)}", http=res.status)
            if path.endswith("/health"):
                body = res.json() if isinstance(res.json(), dict) else {}
                if body.get("status") != "ok" or body.get("daemon") != "agent":
                    return Probe(False, "health shape", http=res.status)
        return Probe(True, "health+sync+today+positions", http=200)

    def probe_morning_n2(self) -> Probe:
        brief = self.wire("GET", "/api/daemon/morning-brief")
        if brief.status != 200:
            return Probe(False, f"morning-brief HTTP {brief.status}", http=brief.status)
        body = json.dumps(
            {
                "declaration_id": str(uuid.uuid4()),
                "rule_id": "stress-soak",
                "fired_at_ms": int(time.time() * 1000),
                "working": {"source": "stress-soak"},
            }
        ).encode("utf-8")
        fire = self.wire("POST", "/api/daemon/journal/condition-fire", body)
        if fire.status != 200:
            return Probe(False, f"condition-fire HTTP {fire.status} {safe_excerpt(fire.body_text)}", http=fire.status)
        parsed = fire.json() if isinstance(fire.json(), dict) else {}
        if parsed.get("ok") is False:
            return Probe(False, "condition-fire ok=false", http=fire.status)
        return Probe(True, "brief+n2", http=200)

    def probe_cite_due(self) -> Probe:
        cites = self.wire("GET", "/api/daemon/journal/trip-cites")
        if cites.status != 200:
            return Probe(False, f"trip-cites HTTP {cites.status}", http=cites.status)
        decls = self.wire("GET", "/api/daemon/bar/declarations?scope=recent")
        if decls.status != 200:
            return Probe(False, f"declarations HTTP {decls.status}", http=decls.status)
        return Probe(True, "cites+due-source", http=200)

    def probe_depth_condition(self) -> Probe:
        q = urllib.parse.urlencode({"instrument": "BTCUSDT", "book": "binance-com-spot"})
        depth = self.open_get(f"/api/station/depth?{q}")
        if depth.status != 200:
            return Probe(False, f"depth HTTP {depth.status}", http=depth.status)
        # Synthetic id: freeze_condition_on_row no-ops when the row is absent.
        cap = json.dumps(
            {
                "declaration_id": str(uuid.uuid4()),
                "last": None,
                "last_status": "unavailable",
            }
        ).encode("utf-8")
        frozen = self.wire("POST", "/api/daemon/bar/capture-working-condition", cap)
        if frozen.status != 200:
            return Probe(
                False,
                f"capture-working-condition HTTP {frozen.status} {safe_excerpt(frozen.body_text)}",
                http=frozen.status,
            )
        fire_body = json.dumps(
            {
                "declaration_id": str(uuid.uuid4()),
                "rule_id": "stress-soak-depth",
                "fired_at_ms": int(time.time() * 1000),
            }
        ).encode("utf-8")
        fire = self.wire("POST", "/api/daemon/journal/condition-fire", fire_body)
        if fire.status != 200:
            return Probe(False, f"condition-fire HTTP {fire.status}", http=fire.status)
        return Probe(True, "depth+condition", http=200)

    def probe_kill(self) -> Probe:
        state = self.wire("GET", "/api/daemon/kill-switch/state")
        if state.status != 200:
            return Probe(False, f"state HTTP {state.status}", http=state.status)
        body = state.json() if isinstance(state.json(), dict) else {}
        if "active" not in body:
            return Probe(False, "state missing active", http=state.status)
        audit = self.wire("GET", "/api/daemon/kill-switch/audit")
        if audit.status != 200:
            return Probe(False, f"audit HTTP {audit.status}", http=audit.status)
        return Probe(True, f"active={bool(body.get('active'))}", http=200)

    def probe_funds_sizer(self) -> Probe:
        sync = self.wire("GET", "/api/daemon/broker/sync-state")
        slug = ""
        if isinstance(sync.json(), dict):
            slug = str(sync.json().get("brokerSlug") or "")
        adapter, book = funds_target(slug)
        q = urllib.parse.urlencode({"adapter": adapter, "book": book, "operation": "funds"})
        funds = self.open_get(f"/api/station/obtain?{q}")
        if funds.status != 200:
            return Probe(False, f"obtain funds HTTP {funds.status}", http=funds.status)
        preview_body = json.dumps(
            {
                "book_id": "binance-com-spot",
                "side": "BUY",
                "budget_mode": "risk_percent",
                "budget_value": 1,
                "entry": 50000,
                "stop": 49000,
                "target": 52000,
                "funds_lit": True,
                "funds_balance": 10000,
            }
        ).encode("utf-8")
        preview = self.wire("POST", "/api/daemon/risk/preview", preview_body)
        if preview.status != 200:
            return Probe(False, f"risk/preview HTTP {preview.status}", http=preview.status)
        parsed = preview.json() if isinstance(preview.json(), dict) else {}
        if not isinstance(parsed.get("reason"), str) and "reason" not in parsed:
            # Shape may nest the reason. Any 200 JSON object is the local sizer.
            if not isinstance(parsed, dict) or not parsed:
                return Probe(False, "risk/preview empty", http=preview.status)
        limits = self.wire("GET", "/api/daemon/bar/profile/loss-limits")
        if limits.status >= 500:
            return Probe(False, f"loss-limits HTTP {limits.status}", http=limits.status)
        if limits.status == 401:
            return Probe(False, "loss-limits SESSION_PROOF", http=limits.status)
        if limits.status != 200 and limits.status != 404:
            return Probe(False, f"loss-limits HTTP {limits.status} {safe_excerpt(limits.body_text)}", http=limits.status)
        return Probe(True, f"funds status-only slug={slug or 'none'}", http=200)

    def probe_totp_read(self) -> Probe:
        res = self.wire("GET", "/api/daemon/broker/sync-state")
        if res.status != 200:
            return Probe(False, f"sync-state HTTP {res.status}", http=res.status)
        return Probe(True, "sync-state", http=200)

    def probe_declare_cancel(self) -> Probe:
        payload = self.lib.build_declare_payload("BTCUSDT", "binance-com-spot", "planned")
        payload["declaration_payload"]["s1"]["intent"] = (
            "Stress soak paper probe — cancel immediately. No broker order."
        )
        payload["declaration_payload"]["s1"]["invalidation"] = "Harness cancel after probe."
        res = self.wire("POST", "/api/daemon/bar/declare", json.dumps(payload).encode("utf-8"))
        body = res.json() if isinstance(res.json(), dict) else {}
        decl_id = body.get("declarationId") or body.get("declaration_id")
        if res.status not in (200, 201) or not decl_id:
            return Probe(False, f"declare HTTP {res.status} {safe_excerpt(res.body_text)}", http=res.status)
        try:
            uuid.UUID(str(decl_id))
        except ValueError:
            return Probe(False, "declare id not uuid", http=res.status)
        cancel_body = json.dumps(
            {"declaration_id": str(decl_id), "cancel_reason_chip": "scratch"}
        ).encode("utf-8")
        cancel = self.wire("POST", "/api/daemon/bar/cancel-declaration", cancel_body)
        if cancel.status not in (200, 201, 204):
            with self.orphan_lock:
                self.orphans.append(str(decl_id))
            return Probe(
                False,
                f"cancel HTTP {cancel.status} {safe_excerpt(cancel.body_text)}",
                http=cancel.status,
            )
        return Probe(True, "declare+cancel", http=cancel.status)

    def probe_capture(self) -> Probe:
        payload = self.lib.capture_accept_payload()
        payload["draftText"] = "Stress soak capture probe — safe to delete."
        res = self.wire(
            "POST",
            "/api/daemon/journal/toolbar-capture/accept",
            json.dumps(payload).encode("utf-8"),
        )
        if res.status not in (200, 202):
            return Probe(False, f"accept HTTP {res.status} {safe_excerpt(res.body_text)}", http=res.status)
        return Probe(True, f"accept HTTP {res.status}", http=res.status)

    def probe_fill(self) -> Probe:
        body = json.dumps(
            {
                "declaration_id": str(uuid.uuid4()),
                "symbol": "BTCUSDT",
                "side": "BUY",
                "quantity": 0.001,
                "price": 50000,
            }
        ).encode("utf-8")
        res = self.wire("POST", "/api/daemon/bar/test/fill-matched", body)
        if res.status == 404:
            return Probe(True, "expected 404 flag off", http=404)
        if 200 <= res.status < 300:
            return Probe(False, "SAFETY fill-matched returned 2xx (flag should be off)", http=res.status)
        return Probe(False, f"fill-matched HTTP {res.status} {safe_excerpt(res.body_text)}", http=res.status)

    def probe_sparkle(self) -> Probe:
        req = urllib.request.Request(APPCAST_URL, method="GET")
        try:
            with urllib.request.urlopen(req, timeout=20) as resp:
                text = resp.read(200_000).decode("utf-8", errors="replace")
                status = resp.status
        except urllib.error.HTTPError as exc:
            return Probe(False, f"appcast HTTP {exc.code}", http=exc.code)
        except Exception as exc:  # noqa: BLE001
            return Probe(False, type(exc).__name__)
        if status != 200:
            return Probe(False, f"appcast HTTP {status}", http=status)
        if "sparkle:version" not in text and "<item" not in text:
            return Probe(False, "appcast missing items", http=status)
        return Probe(True, "appcast 200", http=status)

    def outbox_dead_letters(self) -> Optional[int]:
        res = self.wire("GET", "/api/daemon/journal/toolbar-capture/outbox/status")
        if res.status != 200 or not isinstance(res.json(), dict):
            return None
        body = res.json()
        counts = (body.get("data") or {}).get("counts") or body.get("counts") or {}
        if not isinstance(counts, dict):
            return None
        try:
            return int(counts.get("dead_letter") or 0)
        except (TypeError, ValueError):
            return None

    def due_and_cite_snapshot(self) -> str:
        decls = self.wire("GET", "/api/daemon/bar/declarations?scope=recent")
        cites = self.wire("GET", "/api/daemon/journal/trip-cites")
        items = self.lib.declarations_items(decls.json())
        matched = 0
        due = 0
        for item in items:
            status = str(item.get("status") or item.get("declarationStatus") or "")
            if status != "matched":
                continue
            matched += 1
            if self.lib.notes_post_text(item).strip() == "":
                due += 1
        cite_n = 0
        if isinstance(cites.json(), dict):
            rows = cites.json().get("items")
            if isinstance(rows, list):
                cite_n = len(rows)
        return f"matched={matched} due_empty_post={due} trip_cites_today={cite_n}"

    def status_snapshot(self) -> str:
        health = self.wire("GET", "/api/daemon/health")
        sync = self.wire("GET", "/api/daemon/broker/sync-state")
        hj = health.json() if isinstance(health.json(), dict) else {}
        sj = sync.json() if isinstance(sync.json(), dict) else {}
        caps = sj.get("capabilities") if isinstance(sj.get("capabilities"), dict) else {}
        cap_txt = ",".join(f"{k}={v}" for k, v in sorted(caps.items()) if not SENSITIVE_KEY.search(str(k)))
        return (
            f"version={hj.get('version', '?')} build={hj.get('build', '?')} "
            f"syncState={sj.get('syncState', '?')} brokerSlug={sj.get('brokerSlug', '?')} "
            f"killDnsActive={sj.get('killDnsActive', '?')} caps={cap_txt}"
        )

    def kill_snapshot(self) -> str:
        res = self.wire("GET", "/api/daemon/kill-switch/state")
        body = res.json() if isinstance(res.json(), dict) else {}
        active = body.get("active")
        dns = body.get("dns_active")
        return f"http={res.status} active={active} dns_active={dns}"

    def session_snapshot(self) -> str:
        res = self.wire("GET", "/api/daemon/auth/station/session")
        body = res.json() if isinstance(res.json(), dict) else {}
        signed = body.get("signed_in")
        profile = str(body.get("profile_id") or "")
        prefix = profile[:8] if profile else "-"
        aud = body.get("aud") if isinstance(body.get("aud"), str) else ""
        return f"http={res.status} signed_in={signed} profile_prefix={prefix} aud={aud or '-'}"

    def double_cancel(self) -> str:
        payload = self.lib.build_declare_payload("BTCUSDT", "binance-com-spot", "planned")
        payload["declaration_payload"]["s1"]["intent"] = (
            "Stress soak double-cancel probe — no broker order."
        )
        res = self.wire("POST", "/api/daemon/bar/declare", json.dumps(payload).encode("utf-8"))
        body = res.json() if isinstance(res.json(), dict) else {}
        decl_id = body.get("declarationId") or body.get("declaration_id")
        if res.status not in (200, 201) or not decl_id:
            return f"declare HTTP {res.status} {safe_excerpt(res.body_text)}"
        cancel_body = json.dumps(
            {"declaration_id": str(decl_id), "cancel_reason_chip": "scratch"}
        ).encode("utf-8")
        first = self.wire("POST", "/api/daemon/bar/cancel-declaration", cancel_body)
        second = self.wire("POST", "/api/daemon/bar/cancel-declaration", cancel_body)
        if first.status not in (200, 201, 204):
            with self.orphan_lock:
                self.orphans.append(str(decl_id))
            return f"first cancel HTTP {first.status}"
        if second.status >= 500:
            return f"second cancel HTTP {second.status}"
        return f"first HTTP {first.status}; second HTTP {second.status} (idempotent 4xx ok)"

    def cleanup_orphans(self) -> None:
        with self.orphan_lock:
            pending = list(dict.fromkeys(self.orphans))
        for decl_id in pending:
            body = json.dumps(
                {"declaration_id": decl_id, "cancel_reason_chip": "scratch"}
            ).encode("utf-8")
            try:
                res = self.wire("POST", "/api/daemon/bar/cancel-declaration", body)
            except Exception as exc:  # noqa: BLE001
                self.log(f"orphan cleanup {decl_id} {type(exc).__name__}")
                continue
            self.log(f"orphan cleanup {decl_id} HTTP {res.status}")

    def write_report(self, *, launch_note: str, known_good: str, appcast_note: str, totp_note: str) -> tuple[str, str]:
        day = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        md_path = os.path.join(self.root, "plans", f"STATION-LIVE-STRESS-SOAK-{day}.md")
        json_path = os.path.join(self.root, "plans", f"STATION-LIVE-STRESS-SOAK-{day}.json")
        lines = [
            f"# Station live stress soak — {day}",
            "",
            f"Started `{self.started}` · ended `{utc_now()}`",
            f"Console base from `~/.tradeautopsy/station.env`: `{read_server_base()}`",
            f"Agent `{self.host}:{self.port}`",
            "",
            launch_note,
            "",
            "## Known-good",
            "",
            known_good,
            "",
            "## Break points",
            "",
            "Passed N is the highest concurrency whose wave fully succeeded. "
            "`none (held through concurrency K)` means the path did not fail before the cap.",
            "",
            "| Path | Passed N | First failure |",
            "| --- | ---: | --- |",
        ]
        for row in self.results:
            failure = row.first_failure.replace("|", "/")
            lines.append(f"| {row.path} | {row.passed_n} | {failure} |")
        lines.extend(["", "## Notes", ""])
        lines.append(f"- TOTP 12h: {totp_note}")
        lines.append(f"- Sparkle: {appcast_note}")
        lines.append(
            "- Restart: not performed. External SIGTERM clears `AgentSupervisor.spawnedPID` "
            "and the poll only marks a disconnect; it does not respawn."
        )
        lines.append("- Fill match 404 is the production gate (`BAR_TEST_INJECT_FILL_MATCHED` unset), not a crash.")
        lines.append("- Declare quantity is 0.001 BTCUSDT and each success is cancelled. No broker order is sent.")
        if self.orphans:
            lines.append(f"- Uncancelled declaration ids ({len(self.orphans)}): " + ", ".join(self.orphans))
        lines.extend(["", "## Waves", ""])
        for row in self.results:
            if row.notes:
                lines.append(f"- **{row.path}**: {row.notes}")
            for wave in row.waves:
                lines.append(
                    f"- **{row.path}** n={wave['concurrency']} ok={wave['ok']} "
                    f"fail={wave['fail']} p95={wave['p95_s']}s"
                )
        lines.extend(["", "## Re-run", "", "```bash", "./scripts/station-live-stress-soak.sh", "```", ""])
        lines.extend(["## Log", ""])
        for line in self.log_lines:
            lines.append(f"- {line}")
        text = "\n".join(lines) + "\n"
        # Last-pass scrub in case a probe note still carries a long token.
        text = re.sub(r"(?i)(bearer\s+)[A-Za-z0-9._\-]{12,}", r"\1[redacted]", text)
        open(md_path, "w", encoding="utf-8").write(text)
        payload = {
            "started": self.started,
            "ended": utc_now(),
            "server_base_url": read_server_base(),
            "host": self.host,
            "port": self.port,
            "known_good": known_good,
            "session_card": totp_note,
            "sparkle": appcast_note,
            "orphans": self.orphans,
            "paths": [
                {
                    "path": row.path,
                    "passed_n": row.passed_n,
                    "ok_requests": row.ok_requests,
                    "first_failure": row.first_failure,
                    "notes": row.notes,
                    "waves": row.waves,
                }
                for row in self.results
            ],
        }
        open(json_path, "w", encoding="utf-8").write(json.dumps(redact(payload), indent=2) + "\n")
        return md_path, json_path


def funds_target(slug: str) -> tuple[str, str]:
    if slug == "kotak_neo":
        return ("kotak_neo", "kotak-nse-bse-cash")
    if slug.startswith("binance"):
        return ("binance_com", "binance-com-spot")
    return ("binance_com", "binance-com-spot")


def totp_card_note() -> str:
    """Read local broker metadata timestamps only. Does not touch Keychain or mint."""
    try:
        raw = subprocess.check_output(
            ["defaults", "export", STATION_DEFAULTS_DOMAIN, "-"],
            stderr=subprocess.DEVNULL,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "UserDefaults domain unreadable; mint POST withheld"
    try:
        plist = plistlib.loads(raw)
    except Exception:  # noqa: BLE001
        return "UserDefaults plist unreadable; mint POST withheld"
    rows: list[str] = []
    for key, value in plist.items():
        if not str(key).startswith(METADATA_PREFIX):
            continue
        tail = str(key)[len(METADATA_PREFIX) :]
        parts = tail.split(".")
        slug = parts[1] if len(parts) >= 2 else "?"
        blob = value if isinstance(value, (bytes, bytearray)) else None
        if blob is None:
            continue
        try:
            meta = json.loads(blob.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            rows.append(f"{slug}: metadata present, timestamp unparsed")
            continue
        stamped = meta.get("lastValidatedAt") if isinstance(meta, dict) else None
        if not isinstance(stamped, (int, float)):
            rows.append(f"{slug}: no lastValidatedAt (card stays down)")
            continue
        # JSONEncoder Date is seconds since 2001-01-01 UTC.
        apple_epoch = 978307200
        age = time.time() - (apple_epoch + float(stamped))
        card = age > TOTP_CARD_SECONDS
        rows.append(f"{slug}: age_h={age / 3600:.1f} card_due={card}")
    if not rows:
        return "no station.broker.metadata rows; mint POST withheld"
    return "; ".join(rows) + "; mint POST withheld"


def appcast_version_note() -> str:
    req = urllib.request.Request(APPCAST_URL, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            text = resp.read(200_000).decode("utf-8", errors="replace")
            status = resp.status
    except Exception as exc:  # noqa: BLE001
        return f"fetch {type(exc).__name__}"
    short = re.search(r"sparkle:shortVersionString>([^<]+)", text)
    version = re.search(r"sparkle:version>([^<]+)", text)
    short_s = short.group(1).strip() if short else "?"
    ver_s = version.group(1).strip() if version else "?"
    return f"HTTP {status} newest short={short_s} sparkle:version={ver_s}"


def running_app_version(agent_pid: Optional[int]) -> str:
    if not agent_pid:
        return "agent pid unknown"
    try:
        ppid = subprocess.check_output(
            ["ps", "-p", str(agent_pid), "-o", "ppid="],
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
        parent = subprocess.check_output(
            ["ps", "-p", ppid, "-o", "command="],
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "parent command unread"
    marker = ".app/Contents/MacOS/"
    if marker not in parent:
        return "agent parent is not a Station.app bundle"
    app = parent.split(marker, 1)[0] + ".app"
    plist_path = os.path.join(app, "Contents", "Info.plist")
    try:
        info = plistlib.loads(open(plist_path, "rb").read())
    except OSError:
        return f"plist unread for {os.path.basename(app)}"
    return (
        f"{os.path.basename(app)} "
        f"{info.get('CFBundleShortVersionString', '?')} ({info.get('CFBundleVersion', '?')})"
    )


def launch_station(lib: Any, host: str, port: int, log: Callable[[str], None]) -> str:
    if lib.port_open(host, port):
        return "Agent already listening; soak did not launch Station."
    candidates = [p for p in (*DEBUG_APP_GLOBS, RELEASE_APP) if os.path.isdir(p)]
    if not candidates:
        return "BLOCKER: port 9137 closed and no Station.app bundle found."
    for app in candidates:
        log(f"launching {app}")
        subprocess.run(["open", app], check=False)
        deadline = time.time() + 45
        while time.time() < deadline:
            if lib.port_open(host, port):
                return f"Launched `{app}` and agent accepted connections."
            time.sleep(1)
        log(f"no listener after launching {app}")
    return "BLOCKER: Station launch did not bind port 9137 within 45s."


def known_good_dry_run(lib: Any, root: str, host: str, port: int) -> tuple[str, bool]:
    tmp = tempfile.mkdtemp(prefix="ta-soak-")
    os.makedirs(os.path.join(tmp, "plans"), exist_ok=True)

    def log(msg: str) -> None:
        print(f"known-good {msg}", flush=True)

    report = lib.run_harness(
        root=tmp,
        host=host,
        port=port,
        book="binance-com-spot",
        symbol="BTCUSDT",
        stance="planned",
        dry_run=True,
        skip_capture=False,
        prove_capture=False,
        wait_closed_sec=0,
        inject_matched=False,
        require_debrief=False,
        keep_declaration=False,
        log=log,
    )
    gates = getattr(report, "gates", {}) or {}
    details = getattr(report, "gate_details", {}) or {}
    a = gates.get("A_health", "?")
    b = gates.get("B_station_session", "?")
    c = gates.get("C_sync_state", "?")
    # Drop email if a detail string ever grows one.
    b_detail = str(details.get("B_station_session", ""))
    b_detail = re.sub(r"[\w.+\-]+@[\w.\-]+", "[redacted-email]", b_detail)
    summary = f"Gate A={a} ({details.get('A_health', '')}); Gate B={b} ({b_detail}); Gate C={c} ({details.get('C_sync_state', '')})"
    return summary, a == "PASS"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Paper-only Station live stress soak.")
    parser.add_argument("--root", default=os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=9137)
    parser.add_argument("--max-read", type=int, default=32)
    parser.add_argument("--max-write", type=int, default=16)
    parser.add_argument("--no-launch", action="store_true")
    args = parser.parse_args(argv)

    lib = load_journal_lib(args.root)
    launch_note = "Launch skipped (--no-launch)."
    if not lib.port_open(args.host, args.port):
        if args.no_launch:
            print("BLOCKER: agent not listening on 9137 and --no-launch set", flush=True)
            return 2
        launch_note = launch_station(lib, args.host, args.port, lambda m: print(m, flush=True))
        if launch_note.startswith("BLOCKER"):
            print(launch_note, flush=True)
            return 2

    secret = lib.resolve_daemon_secret(args.port)
    if not secret:
        print("BLOCKER: AGENT_DAEMON_SECRET not in environment or agent process", flush=True)
        return 2

    known, gate_a = known_good_dry_run(lib, args.root, args.host, args.port)
    print(f"known-good {known}", flush=True)

    soak = Soak(
        root=args.root,
        host=args.host,
        port=args.port,
        lib=lib,
        secret=secret,
        max_read=args.max_read,
        max_write=args.max_write,
    )
    soak.log(f"server_base={read_server_base()}")
    soak.log(launch_note)
    soak.log(f"known-good {known}")
    pid = lib.pid_on_port(args.port)
    soak.log(f"agent_pid={pid or 'none'} app={running_app_version(pid)}")

    if not gate_a:
        soak.add_static("known_good", 0, 0, "Gate A failed; ramps skipped")
        md, js = soak.write_report(
            launch_note=launch_note,
            known_good=known,
            appcast_note="not run",
            totp_note="not run",
        )
        print(f"report {md}", flush=True)
        print(f"summary {js}", flush=True)
        return 2

    read_levels = levels_up_to(READ_LEVELS, args.max_read)
    write_levels = levels_up_to(WRITE_LEVELS, args.max_write)

    try:
        soak.log(soak.session_snapshot())
        soak.log(soak.status_snapshot())
        soak.log(soak.kill_snapshot())
    except Exception as exc:  # noqa: BLE001
        soak.log(f"snapshot {type(exc).__name__}")

    soak.ramp("plan_jwt", read_levels, soak.probe_session)
    soak.ramp("station_status", read_levels, soak.probe_status)
    soak.ramp("morning_brief_n2", read_levels, soak.probe_morning_n2)
    try:
        due_note = soak.due_and_cite_snapshot()
    except Exception as exc:  # noqa: BLE001
        due_note = type(exc).__name__
    soak.ramp("journal_cite_due", read_levels, soak.probe_cite_due, notes=due_note)
    soak.ramp("conditions_depth", read_levels, soak.probe_depth_condition)
    soak.ramp("kill_latch_read", read_levels, soak.probe_kill, notes="POST /kill-switch not sent")
    soak.ramp("funds_sizer", read_levels, soak.probe_funds_sizer, notes="balances not logged; preview uses a synthetic 10000")
    totp_note = totp_card_note()
    soak.ramp("totp_12h_sync", read_levels, soak.probe_totp_read, notes=totp_note)
    soak.add_static(
        "sparkle_restart",
        0,
        0,
        "withheld (supervisor does not respawn after external SIGTERM)",
        "feed check is sparkle_feed",
    )
    sparkle_note = appcast_version_note()
    installed = running_app_version(lib.pid_on_port(args.port))
    sparkle_note = f"{sparkle_note}; installed {installed}"
    soak.ramp("sparkle_feed", [1, 4, min(16, args.max_read)], soak.probe_sparkle, notes=sparkle_note)

    # Writes after reads so a Console 429 still leaves the read table intact.
    soak.ramp("declare", write_levels, soak.probe_declare_cancel, notes="each success cancels")
    if soak.agent_up():
        try:
            cancel_note = soak.double_cancel()
        except Exception as exc:  # noqa: BLE001
            cancel_note = type(exc).__name__
        if "second cancel HTTP 5" in cancel_note or cancel_note.startswith("declare HTTP"):
            soak.add_static("cancel", 0, 0, cancel_note)
        else:
            soak.add_static("cancel", 1, 1, f"none ({cancel_note})")
    else:
        soak.add_static("cancel", 0, 0, "agent not listening")

    dead_before = None
    try:
        dead_before = soak.outbox_dead_letters()
    except Exception:  # noqa: BLE001
        dead_before = None
    soak.ramp("capture_ack_outbox", write_levels, soak.probe_capture, notes="draft text marked safe to delete")
    dead_after = None
    try:
        dead_after = soak.outbox_dead_letters()
    except Exception:  # noqa: BLE001
        dead_after = None
    if dead_before is not None and dead_after is not None and dead_after > dead_before:
        soak.add_static(
            "capture_dead_letter",
            0,
            0,
            f"dead_letter grew {dead_before} -> {dead_after}",
        )
    else:
        soak.add_static(
            "capture_dead_letter",
            1 if dead_after is not None else 0,
            1 if dead_after is not None else 0,
            "none" if dead_after is not None else "outbox status unread",
            f"dead_letter before={dead_before} after={dead_after}",
        )

    soak.ramp(
        "fill_match",
        read_levels,
        soak.probe_fill,
        notes="404 is pass (prod flag off). Synthetic declaration id, no real declare.",
    )

    soak.cleanup_orphans()
    md, js = soak.write_report(
        launch_note=launch_note,
        known_good=known,
        appcast_note=sparkle_note,
        totp_note=totp_note,
    )
    print(f"report {md}", flush=True)
    print(f"summary {js}", flush=True)
    broke = any(not r.first_failure.startswith("none") and not r.first_failure.startswith("withheld") for r in soak.results)
    return 1 if broke else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
