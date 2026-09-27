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
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Callable, Optional

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
        if st in ("fresh", "stale", "success") and data is not None:
            return "WORKING"
        if st in ("unavailable", "unknown", "empty", "declared"):
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


def build_declare_payload(symbol: str, book_id: str) -> dict[str, Any]:
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
                "stance": "planned",
                "invalidation": "Harness cancel after probe.",
                "mood_stress": 2.0,
                "mood_impulse": 3.0,
                "mood_frustration": 1.0,
                "mood_excitement": 2.0,
            },
        },
    }


@dataclass
class HarnessReport:
    host: str
    port: int
    dry_run: bool
    book: str
    symbol: str
    gates: dict[str, str] = field(default_factory=dict)
    gate_details: dict[str, str] = field(default_factory=dict)
    tools: list[dict[str, str]] = field(default_factory=list)
    journal: dict[str, str] = field(default_factory=dict)
    outbox: dict[str, str] = field(default_factory=dict)
    errors: list[str] = field(default_factory=list)

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
            "gates": self.gates,
            "gate_details": self.gate_details,
            "tools": self.tools,
            "journal": self.journal,
            "outbox": self.outbox,
            "errors": self.errors,
        }

    def to_markdown(self, day: str) -> str:
        lines = [
            f"# Notch live journal scoreboard — {day}",
            "",
            f"Host `{self.host}:{self.port}` · book `{self.book}` · symbol `{self.symbol}`"
            + (" · **dry-run**" if self.dry_run else ""),
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
        lines.extend(["", "## Tools honesty matrix", "", "| Tool | Score | HTTP | Note |", "| --- | --- | --- | --- |"])
        for row in self.tools:
            lines.append(
                f"| `{row.get('tool', '')}` | **{row.get('score', '')}** | {row.get('http', '')} | {row.get('note', '')} |"
            )
        lines.extend(["", "## Journal declare", ""])
        if self.journal:
            for k, v in self.journal.items():
                lines.append(f"- **{k}**: {v}")
        else:
            lines.append("- (skipped)")
        lines.extend(["", "## Outbox", ""])
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
    dry_run: bool,
    skip_capture: bool,
    log: Callable[[str], None],
) -> HarnessReport:
    report = HarnessReport(host=host, port=port, dry_run=dry_run, book=book, symbol=symbol)

    if not port_open(host, port):
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = f"No listener on {host}:{port}"
        report.errors.append("Gate A: agent not reachable")
        md, js = report.write_artifacts(root)
        log(f"scoreboard: {md}")
        log(f"summary:    {js}")
        return report

    secret = resolve_daemon_secret(port)
    if not secret:
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = "Agent port open but AGENT_DAEMON_SECRET not found (env or process)"
        report.errors.append("Cannot sign wire v1 without daemon secret")
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
    else:
        report.gates["A_health"] = "FAIL"
        report.gate_details["A_health"] = f"HTTP {health.status}"
        report.errors.append("Gate A failed")
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
    else:
        report.gates["B_station_session"] = "AUTH"
        detail = sj.get("error_class") or ("signed_out" if not signed_in else f"HTTP {sess.status}")
        report.gate_details["B_station_session"] = str(detail)
        gate_b_ok = False

    # Gate C
    sync = wire("GET", "/api/daemon/broker/sync-state")
    sync_j = sync.json() if isinstance(sync.json(), dict) else {}
    sync_state = str(sync_j.get("syncState", "?"))
    report.gates["C_sync_state"] = "PASS" if sync.status == 200 else "FAIL"
    slug = sync_j.get("brokerSlug") or "none"
    report.gate_details["C_sync_state"] = f"syncState={sync_state} brokerSlug={slug}"

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
            report.tools.append(
                {"tool": tool, "score": score, "http": str(res.status), "note": note}
            )
        except Exception as exc:  # noqa: BLE001 — harness must continue
            report.tools.append(
                {"tool": tool, "score": "ERROR", "http": "-", "note": str(exc)[:120]}
            )

    # Outbox
    if skip_capture or dry_run:
        report.outbox["status"] = "skipped (--skip-capture or dry-run)"
    else:
        ob = wire("GET", "/api/daemon/journal/toolbar-capture/outbox/status")
        ob_j = ob.json()
        if ob.status == 200 and isinstance(ob_j, dict):
            counts = (ob_j.get("data") or {}).get("counts") or ob_j.get("counts") or {}
            report.outbox["status"] = "ok"
            report.outbox["counts"] = json.dumps(counts)
        else:
            report.outbox["status"] = f"HTTP {ob.status}"

    # Journal declare + cleanup
    if dry_run:
        report.journal["declare"] = "skipped (dry-run)"
    elif not gate_b_ok:
        report.journal["declare"] = "skipped (Gate B AUTH — sign in via Station device login)"
    else:
        payload = build_declare_payload(symbol, book)
        body_bytes = json.dumps(payload).encode("utf-8")
        decl = wire("POST", "/api/daemon/bar/declare", body_bytes)
        dj = decl.json() if isinstance(decl.json(), dict) else {}
        decl_id = dj.get("declarationId") or dj.get("declaration_id")
        if decl.status in (200, 201) and decl_id:
            report.journal["declare"] = f"ok id={decl_id}"
            live = wire("GET", "/api/daemon/bar/live-state")
            lj = live.json() if isinstance(live.json(), dict) else {}
            pending = ((lj.get("notch") or {}).get("pending_declaration")) if lj else None
            if pending:
                report.journal["live_state_pending"] = "yes"
            else:
                report.journal["live_state_pending"] = "no (local book may differ until hydrate)"
            cancel_body = json.dumps(
                {"declaration_id": decl_id, "cancel_reason_chip": "scratch"}
            ).encode("utf-8")
            cancel = wire("POST", "/api/daemon/bar/cancel-declaration", cancel_body)
            report.journal["cancel"] = f"HTTP {cancel.status}"
        else:
            err = dj.get("error_class") or dj.get("message") or decl.body_text[:200]
            report.journal["declare"] = f"fail HTTP {decl.status}: {err}"

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
    parser.add_argument("--host", default=DEFAULT_HOST)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument(
        "--skip-capture",
        action="store_true",
        help="Skip toolbar capture outbox status",
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
        dry_run=args.dry_run,
        skip_capture=args.skip_capture,
        log=log,
    )
    if report.gates.get("A_health") == "FAIL":
        return 2
    if report.errors and report.gates.get("A_health") != "PASS":
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
