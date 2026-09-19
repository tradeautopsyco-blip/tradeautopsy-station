/**
 * PROTOTYPE — throwaway. Locks the dashboard host vs the layout zoo at the file seam.
 * Plan cockpit is glance tiles only. Risk-engine.html stays SL-only A/B/C.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const dash = readFileSync(new URL("./PROTOTYPE-notch-options-dashboard.html", import.meta.url), "utf8");
const zoo = readFileSync(new URL("./PROTOTYPE-notch-risk-engine.html", import.meta.url), "utf8");

describe("plan cockpit has no risk tiles", () => {
  it("options cockpit seed is glance tiles only", () => {
    const block = dash.match(/const OPTIONS_SEEDS[\s\S]*?id: "cockpit"[\s\S]*?tiles: \[([\s\S]*?)\],/);
    assert.ok(block, "cockpit seed must exist");
    const kinds = [...block[1].matchAll(/kind: "([^"]+)"/g)].map((m) => m[1]);
    assert.deepEqual(kinds, ["session", "oi", "payoff", "depth", "chain"]);
  });

  it("asset catalogs do not offer at-stop / at-target tiles", () => {
    assert.equal(dash.includes("RISK_CATALOG"), false);
    assert.equal(/catalog:[\s\S]*?kind: "atStop"/.test(dash), false);
    assert.equal(/catalog:[\s\S]*?kind: "atTarget"/.test(dash), false);
  });
});

describe("layout zoo stays SL-only", () => {
  it("risk-engine file has no TP split", () => {
    assert.equal(zoo.includes("At the target"), false);
    assert.equal(zoo.includes("pathcv-tp"), false);
    assert.equal(zoo.includes("atTarget"), false);
  });
});
