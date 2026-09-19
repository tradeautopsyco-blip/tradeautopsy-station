/**
 * PROTOTYPE — throwaway. Demo sizer for the Pre-trade dashboard host.
 * Nautilus-shaped. Not a docs/reference source. Lift later; delete the HTML around it.
 */
export const ROLES = {
  scalper:  { id: "scalper",  label: "Scalper",  risk: 0.005, maxTrips: 8 },
  intraday: { id: "intraday", label: "Intraday", risk: 0.01,  maxTrips: 5 },
  swing:    { id: "swing",    label: "Swing",    risk: 0.01,  maxTrips: 3 },
  investor: { id: "investor", label: "Investor", risk: 0.02,  maxTrips: null },
};

export function size(input) {
  const reasons = [];
  const role = ROLES[input.role] || ROLES.intraday;
  if (input.seller) {
    return { authoredQty: 0, liveQty: 0, riskMoney: 0, atStop: null, pct: null, gate: "deny",
      reasons: ["Option seller — this measurement is dark. Not a fake max-loss."], riskFrac: role.risk, calmCap: false };
  }
  if (!input.hasStop || input.entry === input.stop) {
    return { authoredQty: 0, liveQty: 0, riskMoney: 0, atStop: null, pct: null, gate: "deny",
      reasons: ["No stop — this card does not invent one. Declare a stop to size."], riskFrac: role.risk, calmCap: false };
  }
  let riskFrac = role.risk;
  const calmCap = input.calm >= 4;
  if (calmCap) { riskFrac *= 0.5; reasons.push("Calm ≥ 4 — risk fraction halved (wired, not copy)."); }
  if (role.maxTrips != null && input.tripsUsed >= role.maxTrips) {
    return { authoredQty: 0, liveQty: 0, riskMoney: 0, atStop: null, pct: 0, gate: "deny",
      reasons: ["Max trips for this role. Size zero."], riskFrac, calmCap };
  }
  const dist = Math.abs(input.entry - input.stop);
  let riskMoney = input.equity * riskFrac;
  const floorLeft = input.floor - input.floorUsed;
  if (floorLeft <= 0) {
    return { authoredQty: 0, liveQty: 0, riskMoney: 0, atStop: null, pct: 0, gate: "deny",
      reasons: ["Daily floor spent. Size zero."], riskFrac, calmCap };
  }
  if (riskMoney > floorLeft) riskMoney = floorLeft;
  let qty = Math.floor((riskMoney / dist) / input.qtyStep) * input.qtyStep;
  if (qty <= 0) {
    return { authoredQty: 0, liveQty: 0, riskMoney, atStop: 0, pct: 0, gate: "deny",
      reasons: ["Stop too wide for this budget — rounds to zero."], riskFrac, calmCap };
  }
  const atStop = qty * dist;
  const pct = atStop / input.equity;
  let gate = "ok";
  const liveQty = input.overrideQty != null ? input.overrideQty : qty;
  if (liveQty * dist - riskMoney > 1e-9) {
    gate = "deny";
    reasons.push("Override is above authored size. Confirm blocked.");
  }
  if (input.fillOpen && !input.hasStop) {
    gate = "deny";
    reasons.push("Open fill with no SL — cannot add.");
  }
  return { authoredQty: qty, liveQty, riskMoney, atStop, pct, gate, reasons, riskFrac, calmCap };
}

/** DualNoBlend TP on the same authored qty. Dark if seller / no stop / no qty. */
export function withTarget(sized, input) {
  const qty = sized.liveQty != null ? sized.liveQty : sized.authoredQty;
  const out = { ...sized, atTarget: null, targetPct: null, rMultiple: null, wrongSideTp: false, targetWarning: null, targetDark: false, targetNote: null };
  if (sized.atStop == null) {
    out.targetDark = true;
    out.targetNote = sized.reasons[0] || null;
    return out;
  }
  if (!(qty > 0 && Number.isFinite(input.tp) && input.tp !== input.entry)) {
    return out;
  }
  const signed = input.side === "buy" ? (input.tp - input.entry) : (input.entry - input.tp);
  out.atTarget = qty * signed;
  out.targetPct = out.atTarget / input.equity;
  if (sized.atStop > 0) out.rMultiple = out.atTarget / sized.atStop;
  const wrong = input.side === "buy" ? input.tp < input.entry : input.tp > input.entry;
  if (wrong) {
    out.wrongSideTp = true;
    out.targetWarning = "Take profit is on the wrong side of entry.";
  }
  return out;
}

