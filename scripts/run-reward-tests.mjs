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
const { creditedSeconds, estimateFocusReward, rewardLabel } = await import(pathToFileURL(output).href);
const session = { rewardVersion: 1, status: "active", plannedDurationSeconds: 1500,
  creditedDurationMs: 600_000, lastTickAt: 1000, startedAt: -3600_000, endedAt: null };
assert.equal(creditedSeconds(session, 3000), 602);
assert.equal(creditedSeconds(session, 900_000), 600, "sleep must not inflate the timer");
assert.equal(creditedSeconds(session, 500), 600, "clock reversal must not inflate the timer");
assert.equal(creditedSeconds({ ...session, status: "completed" }, 3000), 600);
assert.equal(creditedSeconds({ ...session, creditedDurationMs: 1499_000 }, 3000), 1500);
assert.equal(estimateFocusReward(299).parts, 0);
assert.deepEqual(estimateFocusReward(1500), { parts: 200, insight: 12.5, affinityExperience: 0 });
assert.ok(Math.abs(estimateFocusReward(1500, 2).parts - 140) < 1e-9);
assert.equal(estimateFocusReward(1500, 99).parts, 80);
assert.match(rewardLabel({ parts: 160, insight: 10, affinityExperience: 5 }), /亲密经验 \+5/);
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
