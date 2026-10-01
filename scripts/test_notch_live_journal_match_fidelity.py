#!/usr/bin/env python3
"""Unit tests for Wave 0.4 match-fidelity harness helpers (no live agent)."""

import importlib.util
import sys
import unittest
from pathlib import Path

_LIB = Path(__file__).with_name("notch-live-journal-lib.py")
_spec = importlib.util.spec_from_file_location("notch_live_journal_lib", _LIB)
assert _spec and _spec.loader
_nlj = importlib.util.module_from_spec(_spec)
sys.modules["notch_live_journal_lib"] = _nlj
_spec.loader.exec_module(_nlj)

build_fill_matched_inject_body = _nlj.build_fill_matched_inject_body
build_declare_payload = _nlj.build_declare_payload
closed_trip_ready = _nlj.closed_trip_ready
find_journal_trip_cite = _nlj.find_journal_trip_cite


class MatchFidelityHarnessTests(unittest.TestCase):
    def test_build_fill_matched_inject_body_from_declare(self) -> None:
        declare = build_declare_payload("BTCUSDT", "binance-com-spot", "planned")
        body = build_fill_matched_inject_body("decl-1", declare)
        self.assertEqual(body["declaration_id"], "decl-1")
        self.assertEqual(body["symbol"], "BTCUSDT")
        self.assertEqual(body["side"], "BUY")
        self.assertEqual(body["quantity"], 0.001)
        self.assertEqual(body["price"], 50_000.0)

    def test_closed_trip_ready_on_matched_empty_post(self) -> None:
        item = {"status": "matched", "post": ""}
        self.assertTrue(closed_trip_ready(item))

    def test_closed_trip_ready_false_while_pending(self) -> None:
        item = {"status": "pending", "post": ""}
        self.assertFalse(closed_trip_ready(item))

    def test_find_journal_trip_cite_by_declaration_id(self) -> None:
        items = [
            {"declarationId": "a", "net": 1.0, "currency": "USD"},
            {"declaration_id": "b", "net": 2.0, "currency": "INR"},
        ]
        self.assertEqual(find_journal_trip_cite(items, "b")["net"], 2.0)
        self.assertIsNone(find_journal_trip_cite(items, "missing"))


if __name__ == "__main__":
    unittest.main()
