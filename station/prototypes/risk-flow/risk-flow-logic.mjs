/**
 * PROTOTYPE — throwaway. Portable reducer for the capital-preservation flow.
 * Question: when a fill is detected, what must the trader see NOW vs later,
 * and what happens if the live SL ≠ the planned SL, or there was no form?
 *
 * Lift this module; delete the HTML/TUI around it.
 */
export const ACCOUNT = 200_000;
export const ENTRY = 1_300;
export const QTY = 130; // sized so planned SL 1100 ≈ 13% of account

export function riskRupees(entry, sl, qty) {
  if (sl == null || entry == null || qty == null) return null;
  return qty * Math.abs(entry - sl);
}

export function riskPct(rupees, account = ACCOUNT) {
  if (rupees == null) return null;
  return (rupees / account) * 100;
}

/** worse = more account at risk; better = tightened. both get a response. */
export function slDelta(plannedPct, actualPct) {
  if (plannedPct == null || actualPct == null) return { kind: "unknown", deltaPct: null };
  const d = actualPct - plannedPct;
  if (Math.abs(d) < 0.05) return { kind: "same", deltaPct: d };
  if (d > 0) return { kind: "worse", deltaPct: d };
  return { kind: "better", deltaPct: d };
}

export function towardGoal(tradePnl, goalRupees) {
  if (goalRupees == null || goalRupees === 0) return "no-goal";
  if (tradePnl === 0) return "flat";
  const sameSign = Math.sign(tradePnl) === Math.sign(goalRupees);
  return sameSign ? "toward" : "away";
}

export const initialState = {
  scene: "detect", // detect | today | console | notch | circle
  form: "filled", // filled | none
  plannedSl: 1100,
  actualSl: 1150,
  target: 1480,
  tradePnlToday: 900,
  goalRupees: 2000,
  todayToggle: true,
  notchView: "chart", // chart | direct
  declared: false,
  circleMode: "one", // one | many
  openCount: 1,
  detectionOpen: true,
};

export function reduce(state, action) {
  switch (action.type) {
    case "SCENE":
      return { ...state, scene: action.scene };
    case "NO_FORM":
      return { ...state, form: "none", actualSl: null, detectionOpen: true, declared: false };
    case "HAS_FORM":
      return { ...state, form: "filled", plannedSl: 1100, actualSl: 1150, detectionOpen: true };
    case "SL_TIGHTER":
      return { ...state, form: "filled", plannedSl: 1100, actualSl: 1150, detectionOpen: true };
    case "SL_WIDER":
      return { ...state, form: "filled", plannedSl: 1100, actualSl: 1085, detectionOpen: true };
    case "DISMISS_CARD":
      return { ...state, detectionOpen: false };
    case "OPEN_NOTCH":
      return { ...state, scene: "notch", declared: false, detectionOpen: false };
    case "TOGGLE_TODAY":
      return { ...state, todayToggle: !state.todayToggle };
    case "NOTCH_VIEW":
      return { ...state, notchView: action.view };
    case "DECLARE":
      return { ...state, declared: true, form: "filled", plannedSl: state.plannedSl || 1100 };
    case "CIRCLE_MODE":
      return { ...state, circleMode: action.mode, openCount: action.mode === "many" ? 3 : 1 };
    default:
      return state;
  }
}

export function derived(state) {
  const plannedR = state.form === "filled" ? riskRupees(ENTRY, state.plannedSl, QTY) : null;
  const actualR = riskRupees(ENTRY, state.actualSl, QTY);
  const plannedPct = riskPct(plannedR);
  const actualPct = riskPct(actualR);
  const rewardR = state.form === "filled" ? riskRupees(ENTRY, state.target, QTY) : null;
  return {
    plannedR,
    actualR,
    plannedPct,
    actualPct,
    rewardR,
    rewardPct: riskPct(rewardR),
    delta: slDelta(plannedPct, actualPct),
    goal: towardGoal(state.tradePnlToday, state.goalRupees),
    accountPctToday: riskPct(Math.abs(state.tradePnlToday)),
    maxDownsideIsUnknown: state.form === "none" || state.actualSl == null,
  };
}
