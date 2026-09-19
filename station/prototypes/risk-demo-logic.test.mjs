/**
 * PROTOTYPE — throwaway. Locks the dashboard demo engine at the public seam:
 * size() authors qty; withTarget() adds DualNoBlend stop/TP outcomes on that qty.
 * Expected values are the founder-locked worked example, not recomputed from the code.
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { size, withTarget } from "./risk-demo-logic.mjs";

const OPTIONS_BUY = {
  role: "intraday",
  equity: 10000,
  entry: 412.8,
  stop: 380,
  hasStop: true,
  qtyStep: 1,
  calm: 2,
  tripsUsed: 1,
  floor: 600,
  floorUsed: 0,
  overrideQty: null,
  seller: false,
  fillOpen: false,
};

describe("size authors qty from this book", () => {
  it("options buy 412.80 / SL 380 authors qty 3 and 98.40 at the stop", () => {
    const e = size(OPTIONS_BUY);
    assert.equal(e.authoredQty, 3);
    assert.equal(Number(e.atStop.toFixed(2)), 98.4);
    assert.equal((e.pct * 100).toFixed(2), "0.98");
    assert.equal(e.gate, "ok");
  });
});

describe("withTarget uses the same authored qty", () => {
  it("options buy TP 448 is +105.60 USDT (1.06%) on qty 3", () => {
    const e = withTarget(size(OPTIONS_BUY), {
      side: "buy",
      tp: 448,
      entry: 412.8,
      equity: 10000,
    });
    assert.equal(e.authoredQty, 3);
    assert.equal(Number(e.atTarget.toFixed(2)), 105.6);
    assert.equal((e.targetPct * 100).toFixed(2), "1.06");
  });

  it("options buy 448 vs 380 is 1.07R on the same qty", () => {
    const e = withTarget(size(OPTIONS_BUY), {
      side: "buy",
      tp: 448,
      entry: 412.8,
      equity: 10000,
    });
    assert.equal(Number(e.rMultiple.toFixed(2)), 1.07);
  });
});

describe("wrong-side take profit", () => {
  it("buy TP below entry is flagged", () => {
    const e = withTarget(size(OPTIONS_BUY), {
      side: "buy",
      tp: 400,
      entry: 412.8,
      equity: 10000,
    });
    assert.equal(e.wrongSideTp, true);
    assert.equal(e.targetWarning, "Take profit is on the wrong side of entry.");
    assert.equal(e.atTarget < 0, true);
  });

  it("buy TP above entry is not flagged", () => {
    const e = withTarget(size(OPTIONS_BUY), {
      side: "buy",
      tp: 448,
      entry: 412.8,
      equity: 10000,
    });
    assert.equal(e.wrongSideTp, false);
    assert.equal(e.targetWarning, null);
  });
});

describe("seller measurement stays dark", () => {
  it("options seller blanks stop and target and keeps the dark copy on the TP outcome", () => {
    const e = withTarget(size({ ...OPTIONS_BUY, seller: true }), {
      side: "sell",
      tp: 448,
      entry: 412.8,
      equity: 10000,
    });
    assert.equal(e.gate, "deny");
    assert.equal(e.atStop, null);
    assert.equal(e.atTarget, null);
    assert.equal(e.targetPct, null);
    assert.equal(e.rMultiple, null);
    assert.equal(e.targetDark, true);
    assert.equal(e.targetNote, "Option seller — this measurement is dark. Not a fake max-loss.");
  });
});
