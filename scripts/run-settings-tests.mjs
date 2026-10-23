import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";

const nativeRequire = createRequire(import.meta.url);
const code = ts.transpileModule(readFileSync("src/stores/settingsStore.ts", "utf8"), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText;
const server = { enablePetBubble: true, enableSound: false, catSize: 1 };
const requests = [];
const exports = {};
new Function("require", "exports", code)((name) => name === "../services/tauriCommands" ? {
  getAppSettings: async () => ({ ...server }),
  updateAppSettings: (patch) => new Promise((resolve, reject) => requests.push({ patch, resolve, reject })),
} : nativeRequire(name), exports);
const store = exports.useSettingsStore;
const flush = () => new Promise((resolve) => setImmediate(resolve));
const finish = async () => {
  const request = requests.shift();
  Object.assign(server, request.patch);
  store.getState().setSettings({ ...server });
  request.resolve({ ...server });
  await flush();
};
await store.getState().loadSettings();
const first = store.getState().updateSettings({ enablePetBubble: false });
const second = store.getState().updateSettings({ enableSound: true });
await flush();
assert.equal(requests.length, 1, "writes must be serialized");
store.getState().setSettings({ ...server });
assert.equal(store.getState().settings.enablePetBubble, false, "broadcast must not erase a pending toggle");
assert.equal(store.getState().settings.enableSound, true);
await finish();
assert.equal(requests.length, 1);
assert.equal(store.getState().settings.enableSound, true, "earlier response must not erase a newer edit");
await finish();
await Promise.all([first, second]);
assert.deepEqual(store.getState().settings, server);
assert.equal(server.enablePetBubble, false);
assert.equal(server.enableSound, true);

const third = store.getState().updateSettings({ enableSound: false });
await flush();
store.getState().applyOptimistic({ catSize: 1.5 });
await finish();
await third;
assert.equal(store.getState().settings.catSize, 1.5, "slider draft must survive an earlier toggle response");
const fourth = store.getState().updateSettings({ catSize: 1.5 });
await flush();
await finish();
await fourth;
assert.equal(server.catSize, 1.5);

const failed = store.getState().updateSettings({ enableSound: true });
const rejected = assert.rejects(failed, /disk unavailable/);
await flush();
requests.shift().reject(new Error("disk unavailable"));
await rejected;
assert.deepEqual(store.getState().settings, server, "failed save must restore authoritative settings");
const recovery = store.getState().updateSettings({ enablePetBubble: true });
await flush();
await finish();
await recovery;
assert.equal(server.enablePetBubble, true, "a failed write must not block later changes");
console.log("Settings checks passed: serialization, broadcasts, rapid toggles, slider drafts, failed saves, and recovery.");
