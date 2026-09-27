#!/usr/bin/env python3
"""Wire v1 signing + HTTP helpers for notch-live-journal harness (no secrets logged)."""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import re
import secrets
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Callable, Optional

# Plan cockpit tiles counted for hole badge (see STRONG-TESTING §3.4).
HOLE_TILE_TOOLS = ("quote", "history", "depth")

WIRE_PROTO_VERSION = "1"
LOOPBACK_USER_ID = "00000000-0000-4000-8000-000000000002"
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 9137


def new_ulid() -> str:
    """Crockford base32 ULID (128-bit) without external deps."""
    encoding = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    ts_ms = int(time.time() * 1000) & ((1 << 48) - 1)
    rand = int.from_bytes(secrets.token_bytes(10), "big")
    value = (ts_ms << 80) | rand
    chars = []
    for _ in range(26):
        chars.append(encoding[value & 31])
        value >>= 5
    return "".join(reversed(chars))


def rfc3339_millis() -> str:
    return (
        datetime.now(timezone.utc)
        .isoformat(timespec="milliseconds")
        .replace("+00:00", "Z")
    )


def canonical_string(
    method: str, path: str, timestamp: str, request_id: str, body: bytes
) -> str:
    body_hex = hashlib.sha256(body).hexdigest()
    return f"{method.upper()}\n{path}\n{timestamp}\n{request_id}\n{body_hex}"


def sign_request(
    secret: str,
    method: str,
    path: str,
    body: bytes,
    request_id: Optional[str] = None,
    timestamp: Optional[str] = None,
) -> dict[str, str]:
    rid = request_id or new_ulid()
    ts = timestamp or rfc3339_millis()
    nonce = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
    canonical = canonical_string(method, path, ts, rid, body)
    sig = hmac.new(secret.encode("utf-8"), canonical.encode("utf-8"), hashlib.sha256).digest()
    return {
        "x-proto-version": WIRE_PROTO_VERSION,
        "x-daemon-secret": secret,
        "x-user-id": LOOPBACK_USER_ID,
        "x-request-id": rid,
        "x-timestamp": ts,
        "x-nonce": nonce,
        "x-signature": base64.b64encode(sig).decode("ascii"),
    }


def port_open(host: str, port: int, timeout: float = 0.4) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def pid_on_port(port: int) -> Optional[int]:
    try:
        out = subprocess.check_output(
            ["ss", "-ltnp", f"sport = :{port}"],
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        try:
            out = subprocess.check_output(
                ["lsof", "-i", f":{port}", "-t"],
                stderr=subprocess.DEVNULL,
                text=True,
            )
            line = out.strip().splitlines()
            return int(line[0]) if line else None
        except (subprocess.CalledProcessError, FileNotFoundError, ValueError):
            return None
    for line in out.splitlines():
        m = re.search(r"pid=(\d+)", line)
        if m:
            return int(m.group(1))
    return None


def agent_cmdline(pid: int) -> str:
    try:
        raw = open(f"/proc/{pid}/cmdline", "rb").read()
        return raw.replace(b"\0", b" ").decode("utf-8", errors="replace")
    except OSError:
        try:
            out = subprocess.check_output(
                ["ps", "-p", str(pid), "-o", "command="],
                stderr=subprocess.DEVNULL,
                text=True,
            )
            return out.strip()
        except (subprocess.CalledProcessError, FileNotFoundError):
            return ""


def secret_from_pid(pid: int) -> Optional[str]:
    environ_path = f"/proc/{pid}/environ"
    try:
        raw = open(environ_path, "rb").read()
        for entry in raw.split(b"\0"):
            if entry.startswith(b"AGENT_DAEMON_SECRET="):
                val = entry.split(b"=", 1)[1].decode("utf-8", errors="replace")
                return val.strip() or None
    except OSError:
        pass
    # macOS Debug agent: `ps eww -p <pid>` (founder laptop); never log the value.
    try:
        out = subprocess.check_output(
            ["ps", "eww", "-p", str(pid)],
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    for token in out.split():
        if token.startswith("AGENT_DAEMON_SECRET="):
            return token.split("=", 1)[1].strip() or None
    return None


def resolve_daemon_secret(port: int) -> Optional[str]:
    env = os.environ.get("AGENT_DAEMON_SECRET", "").strip()
    if env:
        return env
    pid = pid_on_port(port)
    if pid is None:
        return None
    return secret_from_pid(pid)


@dataclass
class HttpResult:
    status: int
    body_text: str
    path: str
    method: str

    def json(self) -> Any:
        if not self.body_text:
            return None
        try:
            return json.loads(self.body_text)
        except json.JSONDecodeError:
            return None


def http_request(
    host: str,
    port: int,
    method: str,
    path: str,
    *,
    secret: Optional[str] = None,
    body: Optional[bytes] = None,
    wire: bool = True,
    timeout: float = 30.0,
) -> HttpResult:
    body = body if body is not None else b""
    url_path = path.split("?", 1)[0]
    headers: dict[str, str] = {}
    if wire:
        if not secret:
            raise RuntimeError("wire request requires daemon secret")
        headers.update(sign_request(secret, method, url_path, body))
    if body:
        headers["Content-Type"] = "application/json"
    url = f"http://{host}:{port}{path}"
    req = urllib.request.Request(url, data=body or None, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            return HttpResult(resp.status, text, path, method)
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        return HttpResult(e.code, text, path, method)


def score_envelope(body: Any, http_status: int) -> str:
    if http_status == 401:
        return "AUTH"
    if http_status == 403:
        return "AUTH"
    if http_status >= 500:
        return "ERROR"
    if not isinstance(body, dict):
        return "ERROR" if http_status >= 400 else "HONEST-DARK"
    err = body.get("error_class")
    if err in ("SESSION_PROOF", "SIG_INVALID", "AUTH"):
        return "AUTH"
    if err in ("BROKER", "SYNC", "CIRCUIT"):
        return "BROKER"
    if http_status >= 400 and body.get("error_class"):
        return "ERROR"
    status = body.get("status")
    if isinstance(status, str):
        st = status.lower()
        data = body.get("data")
        # STRONG-TESTING §5 LIE — lit status without payload is a hard fail.
        if st in ("fresh", "stale", "success") and data is None:
            return "LIE"
        if st in ("fresh", "stale", "success") and data is not None:
            return "WORKING"
        if st in ("unavailable", "unknown", "empty", "declared", "research_segment"):
            return "HONEST-DARK"
    if body.get("research") is True and body.get("canonical") is False:
        return "HONEST-DARK"
    if body.get("success") is True and body.get("data") is not None:
        return "WORKING"
    symbols = body.get("symbols")
    if isinstance(symbols, list) and symbols:
        return "WORKING"
    if isinstance(symbols, list):
        return "HONEST-DARK"
    if body.get("ltp") is None and http_status == 200:
        return "HONEST-DARK"
    if body.get("ltp") is not None:
        return "WORKING"
    if http_status == 200:
        return "HONEST-DARK"
    return "ERROR"


def build_declare_payload(symbol: str, book_id: str, stance: str) -> dict[str, Any]:
    stance_norm = stance if stance in ("planned", "reactive") else "planned"
    return {
        "symbol": symbol,
        "side": "BUY",
        "quantity": 0.001,
        "stop_loss": 1.0,
        "target_price": 999999.0,
        "declaration_kind": "intraday",
        "book_id": book_id,
        "declaration_payload": {
            "v": 1,
            "protective_sl_consent": False,
            "s1": {
                "setup_type": "harness_probe",
                "intent": "Phase B live journal harness — auto cleanup.",
                "stance": stance_norm,
                "invalidation": "Harness cancel after probe.",
                # BarDeclarationFlowView: calm→mood_stress, confidence→mood_impulse
                "mood_stress": 2.0,
                "mood_impulse": 4.0,
                "mood_frustration": 1.0,
                "mood_excitement": 2.0,
            },
        },
    }


def build_debrief_payload(
    declaration_id: str,
    stance: str,
    *,
    live_note: str = "Harness closed-trip debrief.",
) -> dict[str, Any]:
    """Same shape as `notch/BarPostTradeDebriefPayload.buildJSONObject` + `BarPostTradeView` submit."""
    return {
        "v": 1,
        "declaration_id": declaration_id,
        "moment_a_note": "Harness moment A — outcome note.",
        "adherence": {
            "stop_as_declared": True,
            "size_as_declared": True,
            "invalidation_respected": True,
            "exit_per_plan": True,
            "no_impulsive_add": True,
        },
        "moment_c_context": "cooling",
        "moment_c_note": "Phase B harness debrief — journal sink proof.",
        "completed_at_ms": int(time.time() * 1000),
        "live_note": live_note,
        "emotion_out": 3,
        "stance": stance if stance in ("planned", "reactive") else "planned",
    }


def build_swing_check_in_payload(declaration_id: str, plan_state: str = "GREEN") -> dict[str, Any]:
    """Matches `BarPlanStateView.swingCheckInJSONObject`."""
    return {
        "v": 1,
        "thesis_intact": True,
        "at_ms": int(time.time() * 1000),
        "note": "Harness Working lane proof.",
        "plan_state": plan_state,
        "declaration_id": declaration_id,
    }


def capture_accept_payload() -> dict[str, Any]:
    return {
        "draftText": "Harness capture accept probe — safe to delete.",
        "explicitPending": True,
        "idempotencyKey": f"harness-{new_ulid()}",
    }


def json_top_keys(obj: Any) -> str:
    return ",".join(sorted(obj.keys())[:24]) if isinstance(obj, dict) else ""


def declarations_items(body: Any) -> list[dict[str, Any]]:
    if not isinstance(body, dict):
        return []
    for key in ("items", "declarations"):
        val = body.get(key)
        if isinstance(val, list):
            return [x for x in val if isinstance(x, dict)]
    days = body.get("days")
    if isinstance(days, list):
        out: list[dict[str, Any]] = []
        for day in days:
            if not isinstance(day, dict):
                continue
            for it in day.get("declarations") or day.get("items") or []:
                if isinstance(it, dict):
                    out.append(it)
        return out
    return []


def declaration_id_of(item: dict[str, Any]) -> Optional[str]:
    for key in ("declarationId", "declaration_id", "id"):
        val = item.get(key)
        if isinstance(val, str) and val.strip():
            return val.strip()
    return None


def post_debrief_saved(item: dict[str, Any]) -> bool:
    if item.get("postDebriefComplete") is True or item.get("post_saved") is True:
        return True
    post = item.get("post")
    if isinstance(post, str) and post.strip():
        return True
    if isinstance(post, dict) and any(v for v in post.values() if v not in (None, "", {})):
        return True
    for key in ("moment_c_note", "momentCNote", "debrief"):
        val = item.get(key)
        if isinstance(val, str) and val.strip():
            return True
        if isinstance(val, dict) and val:
            return True
    return False


def closed_trip_ready(item: dict[str, Any]) -> bool:
    """Console journal row with closed trip and empty post (PATCH debrief allowed)."""
    if post_debrief_saved(item):
        return False
    status = str(item.get("status") or item.get("declarationStatus") or "").lower()
    if status in ("cancelled", "expired", "pending"):
        return False
    if item.get("tripClosed") is True or item.get("trip_closed") is True:
        return True
    if item.get("closedAt") or item.get("closed_at_ms") or item.get("closedAtMs"):
        return True
    trip = item.get("trip") or item.get("escrowTrip") or item.get("escrow")
    if isinstance(trip, dict):
        trip_status = str(trip.get("status") or "").lower()
        if trip_status in ("closed", "complete", "filled"):
            return True
        if trip.get("closedAt") or trip.get("closed_at_ms"):
            return True
    if status in ("matched", "closed", "filled", "complete"):
        post = item.get("post")
        if post is None or post == "" or (isinstance(post, dict) and not post):
            return True
    return False


def debrief_journal_proven(item: dict[str, Any], needle: str) -> bool:
    return post_debrief_saved(item) and needle in json.dumps(item)


def fetch_declarations(wire_fn: Callable[..., HttpResult], scope: str = "recent") -> HttpResult:
    return wire_fn("GET", f"/api/daemon/bar/declarations?scope={scope}&limit=50")


def find_declaration_item(wire_fn: Callable[..., HttpResult], decl_id: str) -> Optional[dict[str, Any]]:
    for scope in ("recent", "week", "pending"):
        res = fetch_declarations(wire_fn, scope)
        for item in declarations_items(res.json()):
            if declaration_id_of(item) == decl_id:
                return item
    return None


def wait_for_closed_trip(
    wire_fn: Callable[..., HttpResult],
    decl_id: str,
    wait_sec: int,
    log: Callable[[str], None],
) -> Optional[dict[str, Any]]:
    deadline = time.time() + max(0, wait_sec)
    while True:
        item = find_declaration_item(wire_fn, decl_id)
        if item and closed_trip_ready(item):
            return item
        if wait_sec <= 0:
            return None
        if time.time() >= deadline:
            log(f"Debrief: no closed trip for {decl_id} after {wait_sec}s wait")
            return None
        time.sleep(min(2.0, max(0.5, wait_sec / 5)))
        log(f"Debrief: waiting for closed trip ({int(deadline - time.time())}s left)…")


def parse_debrief_error(body: Any) -> tuple[str, str]:
    if not isinstance(body, dict):
        return ("", str(body)[:120])
    err = str(body.get("error_class") or body.get("error") or "")
    msg = str(body.get("message") or body.get("error") or "")
    return (err, msg)


@dataclass
class HarnessReport:
    host: str
    port: int
    dry_run: bool
    book: str
    symbol: str
    stance: str = "planned"
    gates: dict[str, str] = field(default_factory=dict)
    gate_details: dict[str, str] = field(default_factory=dict)
    tools: list[dict[str, str]] = field(default_factory=list)
    journal: dict[str, str] = field(default_factory=dict)
    working: dict[str, str] = field(default_factory=dict)
    debrief: dict[str, str] = field(default_factory=dict)
    outbox: dict[str, str] = field(default_factory=dict)
    capabilities: dict[str, Any] = field(default_factory=dict)
    holes: int = 0
    errors: list[str] = field(default_factory=list)
    lies: list[str] = field(default_factory=list)

    def gate_pass(self, name: str) -> bool:
        return self.gates.get(name) == "PASS"

    def write_artifacts(self, root: str) -> tuple[str, str]:
        day = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        md_path = os.path.join(root, "plans", f"NOTCH-LIVE-SCOREBOARD-{day}.md")
        json_path = os.path.join(root, "plans", f"NOTCH-LIVE-SCOREBOARD-{day}.json")
        os.makedirs(os.path.dirname(md_path), exist_ok=True)
        with open(json_path, "w", encoding="utf-8") as f:
            json.dump(self.to_json(), f, indent=2)
            f.write("\n")
        with open(md_path, "w", encoding="utf-8") as f:
            f.write(self.to_markdown(day))
            f.write("\n")
        return md_path, json_path

    def to_json(self) -> dict[str, Any]:
        return {
            "generated_at": rfc3339_millis(),
            "host": self.host,
            "port": self.port,
            "dry_run": self.dry_run,
            "book": self.book,
            "symbol": self.symbol,
            "stance": self.stance,
            "gates": self.gates,
            "gate_details": self.gate_details,
            "tools": self.tools,
            "journal": self.journal,
            "working": self.working,
            "debrief": self.debrief,
            "outbox": self.outbox,
            "capabilities": self.capabilities,
            "holes": self.holes,
            "errors": self.errors,
            "lies": self.lies,
        }

    def to_markdown(self, day: str) -> str:
        lines = [
            f"# Notch live journal scoreboard — {day}",
            "",
            f"Host `{self.host}:{self.port}` · book `{self.book}` · symbol `{self.symbol}` · stance `{self.stance}`"
            + (" · **dry-run**" if self.dry_run else ""),
            "",
            f"**Plan cockpit holes** (quote/history/depth dark): **{self.holes}**",
            "",
            "## Gates",
            "",
            "| Gate | Result | Detail |",
            "| --- | --- | --- |",
        ]
        for g in ("A_health", "B_station_session", "C_sync_state"):
            label = {"A_health": "A · health", "B_station_session": "B · station session", "C_sync_state": "C · sync-state"}[g]
            lines.append(
                f"| {label} | {self.gates.get(g, 'SKIP')} | {self.gate_details.get(g, '')} |"
            )
        lines.extend(
            [
                "",
                "## Sync capabilities (Settings broker mirror)",
                "",
            ]
        )
        if self.capabilities:
            for k, v in sorted(self.capabilities.items()):
                lines.append(f"- `{k}`: {v}")
        else:
            lines.append("- (none)")
        lines.extend(
            [
                "",
                "## Tools honesty matrix",
                "",
                "| Tool | Route | Score | HTTP | Note |",
                "| --- | --- | --- | --- | --- |",
            ]
        )
        for row in self.tools:
            lines.append(
                f"| `{row.get('tool', '')}` | `{row.get('route', '')}` | **{row.get('score', '')}** | {row.get('http', '')} | {row.get('note', '')} |"
            )
        if self.lies:
            lines.extend(["", "## LIE (hard fail)", ""])
            for lie in self.lies:
                lines.append(f"- {lie}")
        lines.extend(["", "## Journal · Plan declare", ""])
        if self.journal:
            for k, v in self.journal.items():
                lines.append(f"- **{k}**: {v}")
        else:
            lines.append("- (skipped)")
        lines.extend(["", "## Working (armed / live-state sink)", ""])
        if self.working:
            for k, v in self.working.items():
                lines.append(f"- **{k}**: {v}")
        else:
            lines.append("- (skipped)")
        lines.extend(["", "## Debrief (post-trade PATCH)", ""])
        if self.debrief:
            for k, v in self.debrief.items():
                lines.append(f"- **{k}**: {v}")
        else:
            lines.append("- (skipped)")
        lines.extend(["", "## Toolbar capture / outbox", ""])
        if self.outbox:
            for k, v in self.outbox.items():
                lines.append(f"- **{k}**: {v}")
        if self.errors:
            lines.extend(["", "## Errors", ""])
            for e in self.errors:
                lines.append(f"- {e}")
        return "\n".join(lines)


def run_harness(
    *,
    root: str,
    host: str,
    port: int,
    book: str,
    symbol: str,
    stance: str,
    dry_run: bool,
    skip_capture: bool,
    prove_capture: bool,
    wait_closed_sec: int,
    require_debrief: bool,
    keep_declaration: bool,
    log: Callable[[str], None],
) -> HarnessReport:
    report = HarnessReport(
        host=host,
        port=port,
        dry_run=dry_run,
        book=book,
        symbol=symbol,
        stance=stance,
    )

    if not port_open(host, port):
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = f"No listener on {host}:{port}"
        report.errors.append("Gate A: agent not reachable")
        md, js = report.write_artifacts(root)
        log(f"scoreboard: {md}")
        log(f"summary:    {js}")
        return report

    pid = pid_on_port(port)
    if pid is not None:
        cmd = agent_cmdline(pid)
        if cmd and "tradeautopsy-agent" not in cmd and "target/debug/tradeautopsy-agent" not in cmd:
            log(f"Gate A: warn — listener pid {pid} may not be tradeautopsy-agent")

    secret = resolve_daemon_secret(port)
    if not secret:
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = "Agent port open but AGENT_DAEMON_SECRET not found (env or ps eww)"
        report.errors.append("Cannot sign wire v1 without daemon secret")
        log("Gate A: FAIL (no AGENT_DAEMON_SECRET)")
        md, js = report.write_artifacts(root)
        log(f"scoreboard: {md}")
        log(f"summary:    {js}")
        return report

    def wire(method: str, path: str, body: Optional[bytes] = None) -> HttpResult:
        return http_request(host, port, method, path, secret=secret, body=body, wire=True)

    def open_get(path: str) -> HttpResult:
        return http_request(host, port, "GET", path, secret=None, wire=False)

    # Gate A
    health = wire("GET", "/api/daemon/health")
    if health.status == 200 and isinstance(health.json(), dict):
        report.gates["A_health"] = "PASS"
        hj = health.json()
        report.gate_details["A_health"] = f"ok daemon={hj.get('daemon', '?')} status={hj.get('status', '?')}"
        log("Gate A: PASS")
    else:
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = f"HTTP {health.status}"
        report.errors.append("Gate A failed")
        log("Gate A: FAIL")
        md, js = report.write_artifacts(root)
        log(f"scoreboard: {md}")
        log(f"summary:    {js}")
        return report

    # Gate B
    sess = wire("GET", "/api/daemon/auth/station/session")
    sj = sess.json() if isinstance(sess.json(), dict) else {}
    signed_in = sj.get("signed_in") is True
    session_proof = sj.get("error_class") == "SESSION_PROOF" or sess.status == 401
    if signed_in and not session_proof:
        report.gates["B_station_session"] = "PASS"
        report.gate_details["B_station_session"] = f"profile={sj.get('profile_id', '?')}"
        gate_b_ok = True
        log("Gate B: PASS")
    else:
        report.gates["B_station_session"] = "AUTH"
        detail = sj.get("error_class") or ("signed_out" if not signed_in else f"HTTP {sess.status}")
        report.gate_details["B_station_session"] = str(detail)
        gate_b_ok = False
        log(f"Gate B: AUTH ({detail})")

    # Gate C — report posture; stale sync still allows HONEST-DARK tools (STRONG-TESTING §5).
    sync = wire("GET", "/api/daemon/broker/sync-state")
    sync_j = sync.json() if isinstance(sync.json(), dict) else {}
    sync_state = str(sync_j.get("syncState", "?"))
    report.gates["C_sync_state"] = "PASS" if sync.status == 200 else "FAIL"
    slug = sync_j.get("brokerSlug") or "none"
    caps = sync_j.get("capabilities")
    if isinstance(caps, dict):
        report.capabilities = caps
    stale_note = ""
    if sync_state not in ("fresh", "FRESH", "connected", "CONNECTED"):
        stale_note = " · tools may be HONEST-DARK"
    report.gate_details["C_sync_state"] = f"syncState={sync_state} brokerSlug={slug}{stale_note}"
    log(f"Gate C: {report.gates['C_sync_state']} ({sync_state})")

    # Tools matrix (open loopback extracts + wired instruments)
    q = urllib.parse.urlencode({"instrument": symbol, "book": book})
    tool_specs: list[tuple[str, str, bool]] = [
        ("quote", f"/api/station/quote?{q}", False),
        ("history", f"/api/station/history?{q}&limit=5", False),
        ("depth", f"/api/station/depth?{q}", False),
        ("option_chain", f"/api/station/chain?{q}", False),
        ("open_interest", f"/api/station/oi?{q}", False),
        ("greeks", f"/api/station/greeks?{q}", False),
        ("index", f"/api/station/index?{urllib.parse.urlencode({'instrument': symbol})}", False),
        (
            "instruments_search",
            f"/instruments/search?{urllib.parse.urlencode({'q': symbol[:4] if len(symbol) >= 4 else symbol})}",
            True,
        ),
        (
            "instruments_ltp",
            f"/instruments/ltp?{urllib.parse.urlencode({'symbol': symbol})}",
            True,
        ),
    ]
    for tool, path, needs_wire in tool_specs:
        try:
            res = wire("GET", path) if needs_wire else open_get(path)
            body = res.json()
            score = score_envelope(body, res.status)
            note = ""
            if isinstance(body, dict) and body.get("status"):
                note = f"status={body.get('status')}"
            elif isinstance(body, dict) and body.get("error_class"):
                note = str(body.get("error_class"))
            route = path.split("?", 1)[0]
            report.tools.append(
                {
                    "tool": tool,
                    "route": route,
                    "score": score,
                    "http": str(res.status),
                    "note": note,
                }
            )
            if score == "LIE":
                report.lies.append(f"{tool} {route}: {note or 'lit status without data'}")
                report.errors.append(f"LIE: {tool}")
        except Exception as exc:  # noqa: BLE001 — harness must continue
            report.tools.append(
                {
                    "tool": tool,
                    "route": path.split("?", 1)[0],
                    "score": "ERROR",
                    "http": "-",
                    "note": str(exc)[:120],
                }
            )

    report.holes = sum(
        1
        for row in report.tools
        if row.get("tool") in HOLE_TILE_TOOLS and row.get("score") == "HONEST-DARK"
    )

    decl_id: Optional[str] = None
    declare_payload: Optional[dict[str, Any]] = None

    # Journal declare → Working → Debrief → optional cancel
    if dry_run:
        report.journal["declare"] = "skipped (dry-run)"
        report.working["verdict"] = "skipped (dry-run)"
        report.debrief["verdict"] = "skipped (dry-run)"
    elif not gate_b_ok:
        report.journal["declare"] = "skipped (Gate B AUTH — sign in via Station device login)"
        report.working["verdict"] = "skipped (Gate B)"
        report.debrief["verdict"] = "skipped (Gate B)"
    else:
        declare_payload = build_declare_payload(symbol, book, stance)
        body_bytes = json.dumps(declare_payload).encode("utf-8")
        decl = wire("POST", "/api/daemon/bar/declare", body_bytes)
        dj = decl.json() if isinstance(decl.json(), dict) else {}
        decl_id = dj.get("declarationId") or dj.get("declaration_id")
        report.journal["declare_http"] = str(decl.status)
        if decl.status in (200, 201) and decl_id:
            try:
                uuid.UUID(str(decl_id))
                report.journal["declare"] = f"PASS id={decl_id}"
                report.journal["declare_response_keys"] = json_top_keys(dj)
            except ValueError:
                report.journal["declare"] = f"FAIL invalid declaration id: {decl_id}"
                report.errors.append("declare id not a UUID")
                decl_id = None
        else:
            err = dj.get("error_class") or dj.get("message") or decl.body_text[:200]
            report.journal["declare"] = f"FAIL HTTP {decl.status}: {err}"
            decl_id = None

        if decl_id:
            list_res = fetch_declarations(wire, "recent")
            report.journal["declarations_list_http"] = str(list_res.status)
            lj_list = list_res.json() if isinstance(list_res.json(), dict) else {}
            report.journal["declarations_list_keys"] = json_top_keys(lj_list)
            in_list = any(
                declaration_id_of(it) == decl_id for it in declarations_items(lj_list)
            )
            report.journal["declaration_in_list"] = "yes" if in_list else "no"

            live = wire("GET", "/api/daemon/bar/live-state")
            lj = live.json() if isinstance(lj.json(), dict) else {}
            report.working["live_state_http"] = str(live.status)
            notch = lj.get("notch") if isinstance(lj, dict) else {}
            if isinstance(notch, dict):
                report.working["plan_state"] = str(notch.get("plan_state", ""))
                report.working["sync_state"] = str(notch.get("sync_state", ""))
            report.working["barFeaturesActive"] = str(lj.get("barFeaturesActive", ""))
            pending = (notch or {}).get("pending_declaration") if isinstance(notch, dict) else None
            if isinstance(pending, dict):
                report.working["pending"] = "yes"
                report.working["pending_id"] = str(pending.get("id", ""))
                ps = pending.get("plan_snapshot")
                if isinstance(ps, dict):
                    report.journal["plan_snapshot_stance"] = str(ps.get("stance", ""))
                    report.journal["plan_snapshot_setup"] = str(ps.get("setup_label", ""))
                    report.journal["plan_snapshot_calm"] = str(ps.get("calm_scale", ""))
                    report.journal["plan_snapshot_confidence"] = str(ps.get("confidence_scale", ""))
                    emo = ps.get("emotion_in")
                    if isinstance(emo, dict):
                        report.journal["emotion_in_calm"] = str(emo.get("calm", ""))
                        report.journal["emotion_in_confidence"] = str(emo.get("confidence", ""))
                        report.journal["emotion_in_frustration"] = str(emo.get("frustration", ""))
                        report.journal["emotion_in_excitement"] = str(emo.get("excitement", ""))
                    report.journal["moods_landed"] = (
                        "yes" if ps.get("calm_scale") is not None else "no"
                    )
                if ps and isinstance(ps, dict) and ps.get("stance") != declare_payload["declaration_payload"]["s1"]["stance"]:
                    report.errors.append("plan_snapshot stance mismatch")
            else:
                report.working["pending"] = "no"

            plan_state = str((notch or {}).get("plan_state") or "GREEN")
            swing_body = json.dumps(build_swing_check_in_payload(decl_id, plan_state)).encode("utf-8")
            swing = wire("POST", "/api/daemon/bar/swing-check-in", swing_body)
            sj = swing.json() if isinstance(swing.json(), dict) else {}
            report.working["swing_check_in_http"] = str(swing.status)
            report.working["swing_check_in_ok"] = str(sj.get("ok", ""))
            if swing.status >= 400:
                report.working["swing_check_in_error"] = str(
                    sj.get("message") or sj.get("error_class") or swing.body_text[:120]
                )
            if pending and isinstance(pending, dict):
                report.working["verdict"] = (
                    "PASS (armed pending_declaration with plan_snapshot/emotion_in)"
                )
            else:
                report.working["verdict"] = "HONEST-DARK (no pending_declaration on live-state)"

            closed_item = wait_for_closed_trip(wire, decl_id, wait_closed_sec, log)
            debrief_needle = "Phase B harness debrief"
            if closed_item is None:
                report.debrief["verdict"] = (
                    "HONEST-BLOCK (no closed trip — trade flat/round-trip on Console first; "
                    "use --wait-closed-sec after a real fill)"
                )
                report.debrief["http"] = "skipped"
            else:
                debrief_body = json.dumps(
                    build_debrief_payload(decl_id, stance)
                ).encode("utf-8")
                deb = wire("PATCH", "/api/daemon/bar/post-trade-debrief", debrief_body)
                dbj = deb.json() if isinstance(deb.json(), dict) else {}
                report.debrief["http"] = str(deb.status)
                report.debrief["keys"] = json_top_keys(dbj)
                err_cls, err_msg = parse_debrief_error(dbj)
                report.debrief["error_class"] = err_cls
                report.debrief["message"] = err_msg
                report.debrief["ok"] = str(dbj.get("ok", ""))
                if deb.status in (200, 201, 204) and dbj.get("ok") is not False:
                    read_back = find_declaration_item(wire, decl_id) or closed_item
                    if debrief_journal_proven(read_back, debrief_needle):
                        report.debrief["verdict"] = "PASS (journal read-back shows post/debrief fields)"
                        report.debrief["read_back"] = "yes"
                    else:
                        report.debrief["verdict"] = (
                            "HONEST-DARK (PATCH ok but journal read-back inconclusive — "
                            "check Console declarations scope)"
                        )
                        report.debrief["read_back"] = "inconclusive"
                elif deb.status == 400 and "invalid_body" in err_msg.lower():
                    report.debrief["verdict"] = (
                        "HONEST-BLOCK HTTP 400 invalid_body (route reached; "
                        "closed trip or body gate on Console)"
                    )
                else:
                    report.debrief["verdict"] = f"FAIL HTTP {deb.status}: {err_msg or err_cls}"

            if require_debrief and not report.debrief.get("verdict", "").startswith("PASS"):
                report.errors.append("require-debrief: debrief did not PASS")

            if not keep_declaration and decl_id:
                cancel_body = json.dumps(
                    {"declaration_id": decl_id, "cancel_reason_chip": "scratch"}
                ).encode("utf-8")
                cancel = wire("POST", "/api/daemon/bar/cancel-declaration", cancel_body)
                cj = cancel.json() if isinstance(cancel.json(), dict) else {}
                report.journal["cancel"] = f"HTTP {cancel.status}"
                report.journal["cancel_error"] = str(
                    cj.get("message") or cj.get("error_class") or ""
                )
                live_after = wire("GET", "/api/daemon/bar/live-state")
                la = live_after.json() if isinstance(live_after.json(), dict) else {}
                pending_after = ((la.get("notch") or {}).get("pending_declaration")) if la else None
                report.journal["pending_after_cancel"] = (
                    "cleared" if not pending_after else "still_set"
                )
            elif keep_declaration:
                report.journal["cancel"] = "skipped (--keep-declaration)"

    # Outbox status (+ optional accept proof from Mac E2E)
    if skip_capture and not prove_capture:
        report.outbox["status"] = "skipped (--skip-capture)"
    else:
        ob0 = wire("GET", "/api/daemon/journal/toolbar-capture/outbox/status")
        ob0_j = ob0.json() if isinstance(ob0.json(), dict) else {}
        if ob0.status == 200:
            counts0 = (ob0_j.get("data") or {}).get("counts") or ob0_j.get("counts") or {}
            report.outbox["baseline_status"] = "ok"
            report.outbox["baseline_counts"] = json.dumps(counts0)
        else:
            report.outbox["baseline_status"] = f"HTTP {ob0.status}"

        if prove_capture and gate_b_ok and not dry_run:
            acc_body = json.dumps(capture_accept_payload()).encode("utf-8")
            acc = wire("POST", "/api/daemon/journal/toolbar-capture/accept", acc_body)
            aj = acc.json() if isinstance(acc.json(), dict) else {}
            report.outbox["accept_http"] = str(acc.status)
            report.outbox["accept_keys"] = json_top_keys(aj)
            report.outbox["accept_success"] = str(aj.get("success", ""))
            if acc.status >= 400:
                report.outbox["accept_error"] = str(
                    aj.get("message") or aj.get("error_class") or acc.body_text[:120]
                )
            report.outbox["accept_verdict"] = (
                "PASS (enqueued/acked)" if acc.status in (200, 202) else f"HTTP {acc.status}"
            )

        ob1 = wire("GET", "/api/daemon/journal/toolbar-capture/outbox/status")
        ob1_j = ob1.json() if isinstance(ob1.json(), dict) else {}
        if ob1.status == 200:
            counts1 = (ob1_j.get("data") or {}).get("counts") or ob1_j.get("counts") or {}
            report.outbox["after_counts"] = json.dumps(counts1)
            report.outbox["status_verdict"] = "ok"
        else:
            report.outbox["status_verdict"] = f"HTTP {ob1.status}"

    md, js = report.write_artifacts(root)
    log(f"scoreboard: {md}")
    log(f"summary:    {js}")
    return report


def main(argv: list[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(
        description="Phase B Notch live journal harness (wire-v1, gates, tools matrix)."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Skip declare/cancel/outbox (gates + read-only tool probes still run)",
    )
    parser.add_argument("--book", default="binance-com-spot", help="TickBook / desk book id")
    parser.add_argument("--symbol", default="BTCUSDT", help="Instrument for probes and declare")
    parser.add_argument(
        "--stance",
        default="planned",
        choices=("planned", "reactive"),
        help="Plan declare stance (matches Notch Planned/Reactive)",
    )
    parser.add_argument("--host", default=DEFAULT_HOST)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument(
        "--skip-capture",
        action="store_true",
        help="Skip toolbar capture outbox status (and accept proof)",
    )
    parser.add_argument(
        "--prove-capture",
        action="store_true",
        help="POST toolbar-capture accept after baseline outbox (Mac E2E lane)",
    )
    parser.add_argument(
        "--wait-closed-sec",
        type=int,
        default=0,
        help="Poll Console declarations for closed trip before Debrief (0 = single check, no wait)",
    )
    parser.add_argument(
        "--require-debrief",
        action="store_true",
        help="Exit 4 unless Debrief PASS with journal read-back",
    )
    parser.add_argument(
        "--keep-declaration",
        action="store_true",
        help="Do not cancel declaration after run (use when waiting for real fill)",
    )
    parser.add_argument(
        "--root",
        default=os.path.abspath(os.path.join(os.path.dirname(__file__), "..")),
        help="Repo root for scoreboard output",
    )
    args = parser.parse_args(argv)

    def log(msg: str) -> None:
        print(msg, flush=True)

    report = run_harness(
        root=args.root,
        host=args.host,
        port=args.port,
        book=args.book,
        symbol=args.symbol,
        stance=args.stance,
        dry_run=args.dry_run,
        skip_capture=args.skip_capture,
        prove_capture=args.prove_capture,
        wait_closed_sec=args.wait_closed_sec,
        require_debrief=args.require_debrief,
        keep_declaration=args.keep_declaration,
        log=log,
    )
    if report.gates.get("A_health") == "FAIL":
        return 2
    if report.lies:
        return 3
    if args.require_debrief and any(
        e.startswith("require-debrief") for e in report.errors
    ):
        return 4
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
