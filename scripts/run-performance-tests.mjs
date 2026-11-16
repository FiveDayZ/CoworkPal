import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import ts from "typescript";
const nativeRequire = createRequire(import.meta.url);
function load(file, mocks = {}) {
  const exports = {};
  const code = ts.transpileModule(readFileSync(file, "utf8"), { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText;
  const require = (name) => name in mocks ? mocks[name] : name.startsWith(".")
    ? load(path.resolve(path.dirname(file), `${name}.ts`), mocks) : nativeRequire(name);
  new Function("require", "exports", code)(require, exports);
  return exports;
}
const { localDateKey } = load("src/services/dateKeys.ts");
const timezone = process.env.TZ;
try {
  for (const [zone, timestamp, expected] of [
    ["Asia/Shanghai", "2026-11-06T16:30:00Z", "2026-11-07"],
    ["America/Los_Angeles", "2026-01-01T00:30:00Z", "2025-12-31"],
    ["Pacific/Kiritimati", "2024-02-28T12:30:00Z", "2024-02-29"],
    ["America/New_York", "2026-11-01T05:30:00Z", "2026-11-01"],
  ]) {
    process.env.TZ = zone;
    assert.equal(localDateKey(Date.parse(timestamp)), expected, zone);
  }
} finally { if (timezone == null) delete process.env.TZ; else process.env.TZ = timezone; }

const { effectiveCompanionSettings } = load("src/services/companionSettings.ts");
const original = { companionMode: "normal", enableSound: true, enableNotifications: true,
  enablePetBubble: true, enableStaticCatMode: false, enableLowPowerMode: false };
assert.equal(effectiveCompanionSettings(original), original);
const quiet = effectiveCompanionSettings({ ...original, companionMode: "quiet" });
assert.equal(quiet.enableSound, false);
assert.equal(quiet.enableNotifications, false);
assert.equal(quiet.enablePetBubble, false);
assert.equal(quiet.enableStaticCatMode, true);
assert.equal(quiet.enableLowPowerMode, false);
assert.equal(effectiveCompanionSettings({ ...original, companionMode: "economy" }).enableLowPowerMode, true);
assert.equal(original.enableSound, true, "temporary mode must not mutate preferences");
assert.equal(effectiveCompanionSettings(null), null);

const storage = { value: null, getItem() { return this.value; }, setItem(_key, value) { this.value = value; } };
const { readDismissedSuggestions, dismissSuggestion } = load("src/services/suggestionFeedback.ts");
assert.deepEqual(readDismissedSuggestions(storage, "2026-11-16"), []);
dismissSuggestion(storage, "2026-11-16", "focus_gap");
dismissSuggestion(storage, "2026-11-16", "focus_gap");
assert.deepEqual(readDismissedSuggestions(storage, "2026-11-16"), ["focus_gap"]);
assert.deepEqual(readDismissedSuggestions(storage, "2026-11-17"), []);
for (let i = 0; i < 30; i++) dismissSuggestion(storage, "2026-11-16", `item${i}`);
assert.equal(readDismissedSuggestions(storage, "2026-11-16").length, 20);
storage.value = "broken";
assert.throws(() => readDismissedSuggestions(storage, "2026-11-16"));
assert.throws(() => dismissSuggestion({ ...storage, value: null, setItem() { throw Error("quota"); } }, "today", "a"), /quota/);

const { decorationProgress } = load("src/services/workshopDecorations.ts");
assert.deepEqual(decorationProgress(null, []), { none: 1, orders: 0, focus: 0, weekly: 0 });
const finished = { status: "completed", rewardVersion: 1, plannedDurationSeconds: 1500, creditedDurationMs: 300000 };
const progress = decorationProgress({ completedOrderCount: 4, rewardReceipts: { "week:2026:all": {} } },
  [finished, finished, finished, { ...finished, status: "abandoned" }]);
assert.deepEqual(progress, { none: 1, orders: 3, focus: 3, weekly: 1 });
assert.equal(decorationProgress({ completedOrderCount: 0, rewardReceipts: { "week:a:one": {} } },
  [{ ...finished, creditedDurationMs: 299000 }]).focus, 0);

let resolveBook;
const commands = { getFocusSessions: () => new Promise((resolve) => { resolveBook = resolve; }),
  updateFocusOutcome: async () => { throw Error("save failed"); } };
const focus = load("src/stores/focusStore.ts", {
  "../services/tauriCommands": commands, "./workshopStore": { useWorkshopStore: { getState: () => ({}) } },
}).useFocusStore;
const active = { id: "active", status: "active", startedAt: 100, lastTickAt: 200 };
const loadPending = focus.getState().load();
focus.getState().setActiveTick({ ...active, lastTickAt: 500 });
resolveBook({ schemaVersion: 1, sessions: [active, { id: "old", status: "completed" }] });
await loadPending;
assert.equal(focus.getState().book.sessions.length, 2, "tick during initial read must retain loaded history");
assert.equal(focus.getState().activeSession.lastTickAt, 500);
focus.getState().setActiveTick({ ...active, lastTickAt: 300 });
assert.equal(focus.getState().activeSession.lastTickAt, 500, "older ticks must be ignored");
const completed = { schemaVersion: 1, sessions: [{ ...active, status: "completed" }] };
focus.getState().setBook(completed);
focus.getState().setActiveTick({ ...active, lastTickAt: 900 });
assert.equal(focus.getState().activeSession, null, "terminal sessions must not be resurrected");
await assert.rejects(focus.getState().saveOutcome("active", "result"), /save failed/);
assert.equal(focus.getState().book, completed, "failed saves must preserve the last committed book");

const workshopCommands = { getWorkshopUpgradeQuotes: async () => ({}), getWorkshopProductionBreakdown: async () => ({}) };
const workshop = load("src/stores/workshopStore.ts", { "../services/tauriCommands": workshopCommands }).useWorkshopStore;
const state = { revision: 1, lastProductionTime: 100, parts: 100, insight: 10,
  workshopLevel: 1, moduleLevels: {}, rewardReceipts: { old: {} }, completedOrderIds: ["old"] };
workshop.getState().setTick({ revision: 1, lastProductionTime: 90, parts: 90 });
assert.equal(workshop.getState().state, null);
workshop.getState().setWorkshopState(state);
workshop.getState().setTick({ revision: 1, lastProductionTime: 101, parts: 101 });
assert.equal(workshop.getState().state.parts, 101);
assert.deepEqual(workshop.getState().state.rewardReceipts, { old: {} });
assert.deepEqual(workshop.getState().state.completedOrderIds, ["old"]);
workshop.getState().setTick({ revision: 2, lastProductionTime: 102, parts: 999 });
assert.equal(workshop.getState().state.parts, 101, "tick must wait for its complete economic state");
workshop.getState().setWorkshopState({ ...state, revision: 2, parts: 600, rewardReceipts: { old: {}, reward: {} } });
workshop.getState().setTick({ revision: 1, lastProductionTime: 105, parts: 102 });
assert.equal(workshop.getState().state.parts, 600, "delayed tick must not erase a reward");
workshop.getState().setTick({ revision: 2, lastProductionTime: 110, parts: 601 });
workshop.getState().setWorkshopState({ ...state, revision: 2, parts: 600 });
assert.equal(workshop.getState().state.parts, 601, "older same-generation read must not erase newer production");
workshop.getState().setWorkshopState(state);
assert.equal(workshop.getState().state.revision, 2);

let refIndex = 0;
const refs = [];
let previousDeps;
let records = 0;
let repeatEffect = false;
const { useCoCatPerformance } = load("src/pet/cocat/performance/useCoCatPerformance.ts", {
  react: {
    useRef(value) { const i = refIndex++; return refs[i] ??= { current: value }; },
    useState(initial) { return [typeof initial === "function" ? initial() : initial, () => {}]; },
    useEffect(effect, deps) {
      if (!previousDeps || deps.some((value, i) => !Object.is(value, previousDeps[i]))) {
        effect(); if (repeatEffect) effect(); previousDeps = deps;
      }
    },
  },
  "./coCatPerformanceMonitor": { createInitialCoCatPerformanceReport: () => ({}),
    createCoCatPerformanceMonitor: () => ({ recordFrame() { records++; return null; } }) },
});
function render(at) { refIndex = 0; useCoCatPerformance({ animationState: "idle" }, at); }
render(0);
repeatEffect = true;
for (let i = 1; i <= 12; i++) { render(i * 83); render(i * 83); }
assert.equal(records, 12, "every new animation sample counts once even if its input is unchanged or effects replay");
console.log("Performance/feature checks passed: local dates, scenes, feedback, decorations, focus races, reward-safe ticks, and animation sampling.");
