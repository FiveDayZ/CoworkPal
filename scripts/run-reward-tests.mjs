import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
import ts from "typescript";
import { createRequire } from "node:module";

const root = process.cwd();
const output = path.join(root, ".tmp", "reward-tests", "rewards.mjs");
mkdirSync(path.dirname(output), { recursive: true });
writeFileSync(output, ts.transpileModule(
  readFileSync(path.join(root, "src/types/rewards.ts"), "utf8"),
  { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } },
).outputText);
const { creditedSeconds, estimateFocusReward, rewardLabel, focusRewardLabel } = await import(pathToFileURL(output).href);
const session = { rewardVersion: 1, status: "active", plannedDurationSeconds: 1500,
  creditedDurationMs: 600_000, lastTickAt: 1000, startedAt: -3600_000, endedAt: null };
assert.equal(creditedSeconds(session, 3000), 602);
assert.equal(creditedSeconds(session, 900_000), 600, "sleep must not inflate the timer");
assert.equal(creditedSeconds(session, 500), 600, "clock reversal must not inflate the timer");
assert.equal(creditedSeconds({ ...session, status: "completed" }, 3000), 600);
assert.equal(creditedSeconds({ ...session, creditedDurationMs: 1499_000 }, 3000), 1500);
assert.equal(estimateFocusReward(299).parts, 0);
assert.deepEqual(estimateFocusReward(1500), { parts: 600, insight: 37.5, affinityExperience: 0 });
assert.deepEqual(estimateFocusReward(10800), { parts: 4320, insight: 270, affinityExperience: 0 });
assert.ok(Math.abs(estimateFocusReward(1500, 2).parts - 420) < 1e-9);
assert.equal(estimateFocusReward(1500, 99).parts, 240);
assert.match(rewardLabel({ parts: 160, insight: 10, affinityExperience: 5 }), /亲密经验 \+5/);
const completed = { ...session, status: "completed", rewardPaid: true,
  reward: { parts: 200, insight: 12.5, affinityExperience: 5 } };
assert.equal(focusRewardLabel(completed), "已到账 · 零件 +200 · 灵感 +12.5 · 亲密经验 +5");
assert.match(focusRewardLabel({ ...completed, rewardPaid: false }), /^待到账/);
assert.equal(focusRewardLabel({ ...completed, reward: null }), "奖励待结算");
assert.equal(focusRewardLabel({ ...completed, reward: null, rewardVersion: 0 }), "历史奖励未记录");
assert.equal(focusRewardLabel({ ...completed, status: "abandoned" }), "无完成奖励");
assert.match(focusRewardLabel({ ...completed, reward: estimateFocusReward(299) }), /无完成奖励/);
const workshopOutput = path.join(path.dirname(output), "workshop.mjs");
writeFileSync(workshopOutput, ts.transpileModule(readFileSync(path.join(root, "src/types/workshop.ts"), "utf8"),
  { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText);
const { workshopLevelSteps, workshopCoolingRelief, addAffinityExperience } = await import(pathToFileURL(workshopOutput).href);
for (const [level, expected] of [[1, 0], [100, 99], [101, 99.75], [200, 174], [201, 174.5], [255, 201.5], [256, 201.5]]) {
  assert.equal(workshopLevelSteps(level), expected);
}
assert.equal(workshopCoolingRelief({ parts: 1, process: 1 }), 0);
for (let level = 2; level <= 255; level++) {
  const relief = workshopCoolingRelief({ parts: level, process: level });
  assert.ok(relief > workshopCoolingRelief({ parts: level - 1, process: level - 1 }));
  assert.ok(relief < 0.58, "displayed cooling relief must follow the actual diminishing cap");
}
const workshop = { catAffinityLevel: 99, affinityExperience: 95 };
addAffinityExperience(workshop, 18);
assert.deepEqual(workshop, { catAffinityLevel: 100, affinityExperience: 0 });
addAffinityExperience(workshop, 1000);
assert.deepEqual(workshop, { catAffinityLevel: 100, affinityExperience: 0 });
const storeCode = ts.transpileModule(readFileSync(path.join(root, "src/stores/petStore.ts"), "utf8"),
  { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
const storeExports = {};
const nativeRequire = createRequire(import.meta.url);
new Function("require", "exports", storeCode)(
  (name) => name === "../types/rewards" ? { rewardLabel } : nativeRequire(name), storeExports);
const store = storeExports.usePetStore;
const before = store.getState().catMessage;
const notice = { rewardId: "focus:a", title: "专注完成奖励",
  reward: { parts: 200, insight: 12.5, affinityExperience: 5 } };
store.getState().showReward(notice, false);
assert.equal(store.getState().catMessage, before, "disabled bubbles must remain silent");
store.getState().showReward(notice, true);
assert.match(store.getState().catMessage, /专注完成奖励已到账/);
store.getState().showReward(notice, true);
assert.equal(store.getState().recentReward.parts, 200, "duplicate notices must not accumulate");
store.getState().showReward({ ...notice, rewardId: "week:a" }, true);
assert.equal(store.getState().recentReward.parts, 400);
store.getState().setPetStatus({ catState: "Idle", catMessage: "normal status", timestamp: Date.now() });
assert.match(store.getState().catMessage, /零件 \+400/, "sampling must not overwrite an active reward bubble");
store.getState().setPetStatus({ catState: "TemperatureCheck", catMessage: "heat warning", timestamp: Date.now() });
assert.equal(store.getState().catMessage, "heat warning", "alerts must take precedence");
store.getState().showReward({ ...notice, rewardId: "week:alert" }, true);
assert.equal(store.getState().catMessage, "heat warning", "new rewards must not replace alerts");
store.setState({ rewardMessageUntil: 0 });
store.getState().setPetStatus({ catState: "Idle", catMessage: "normal status", timestamp: Date.now() });
assert.equal(store.getState().catMessage, "normal status");
console.log("Reward checks passed: timing, amounts, bubble toggle, burst totals, deduplication, hold, alert priority, and expiry.");
