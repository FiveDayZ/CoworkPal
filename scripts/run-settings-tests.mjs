import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";

const nativeRequire = createRequire(import.meta.url);
const code = ts.transpileModule(readFileSync("src/stores/settingsStore.ts", "utf8"), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText;
const server = { launchAtStartup: false, enablePetBubble: true, enableSound: false, catSize: 1, catOpacity: 0.95 };
let failReads = false;
const requests = [];
const exports = {};
new Function("require", "exports", code)((name) => name === "../services/tauriCommands" ? {
  getAppSettings: async () => {
    if (failReads) throw new Error("IPC unavailable");
    return { ...server };
  },
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

const failedToggle = store.getState().updateSettings({ launchAtStartup: true });
const failedToggleCheck = assert.rejects(failedToggle, /startup denied/);
const nextToggle = store.getState().updateSettings({ enablePetBubble: false });
await flush();
requests.shift().reject(new Error("startup denied"));
await failedToggleCheck;
await flush();
assert.equal(store.getState().settings.launchAtStartup, false, "failed field must roll back even with a newer request");
assert.deepEqual(requests[0].patch, { enablePetBubble: false }, "failed startup must not be retried by an unrelated toggle");
await finish();
await nextToggle;
assert.equal(server.enablePetBubble, false);

const firstSameField = store.getState().updateSettings({ enableSound: true });
const firstSameFieldCheck = assert.rejects(firstSameField, /first save failed/);
const lastSameField = store.getState().updateSettings({ enableSound: false });
await flush();
requests.shift().reject(new Error("first save failed"));
await firstSameFieldCheck;
assert.equal(store.getState().settings.enableSound, false, "failed earlier save must preserve the newer value of the same field");
await flush();
await finish();
await lastSameField;

store.getState().applyOptimistic({ catSize: 1.25 });
const toggleWithDraft = store.getState().updateSettings({ enableSound: true });
await flush();
assert.deepEqual(requests[0].patch, { enableSound: true }, "a toggle must not commit an unfinished slider draft");
await finish();
await toggleWithDraft;
assert.equal(store.getState().settings.catSize, 1.25);
assert.equal(server.catSize, 1.5);
const commitDraft = store.getState().updateSettings({ catSize: 1.25 });
await flush();
await finish();
await commitDraft;
assert.deepEqual(store.getState().settings, server);
console.log("Settings checks passed: serialization, broadcasts, rapid toggles, slider drafts, failed saves, and recovery.");

const failedWithDraft = store.getState().updateSettings({ enableSound: !server.enableSound });
const failedWithDraftCheck = assert.rejects(failedWithDraft, /save failed/);
store.getState().applyOptimistic({ catOpacity: 0.5 });
await flush();
failReads = true;
requests.shift().reject(new Error("save failed"));
await failedWithDraftCheck;
assert.equal(store.getState().settings.enableSound, server.enableSound, "failed IPC readback must use the last confirmed state");
assert.equal(store.getState().settings.catOpacity, 0.5, "failed toggle must preserve a newer slider draft");
failReads = false;
const finishDraft = store.getState().updateSettings({ catOpacity: 0.5 });
await flush();
await finish();
await finishDraft;
assert.deepEqual(store.getState().settings, server);

const dialogCode = ts.transpileModule(readFileSync("src/services/dialogs.ts", "utf8"), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText;
const dialogExports = {};
new Function("exports", dialogCode)(dialogExports);
let activeDialog;
class FakeElement extends EventTarget {
  children = [];
  returnValue = "";
  append(...elements) { this.children.push(...elements); }
  setAttribute() {}
  remove() { if (activeDialog === this) activeDialog = null; }
  showModal() { activeDialog = this; }
  close(value = "") { this.returnValue = value; this.dispatchEvent(new Event("close")); }
}
globalThis.document = {
  createElement: () => new FakeElement(),
  body: { append() {} },
};
let authorized = false;
const canceled = dialogExports.confirmAction("test").then((value) => { authorized = value; });
await flush();
assert.equal(authorized, false, "an unanswered dialog must not authorize an action");
activeDialog.children[2].children[0].onclick();
await canceled;
assert.equal(authorized, false, "cancel must prevent the action");
assert.equal(activeDialog, null, "a resolved dialog must be removed");
const accepted = dialogExports.confirmAction("test");
activeDialog.children[2].children[1].onclick();
assert.equal(await accepted, true);
const escaped = dialogExports.confirmAction("test");
activeDialog.close();
assert.equal(await escaped, false, "Escape must cancel");
const showModal = FakeElement.prototype.showModal;
FakeElement.prototype.showModal = () => { throw new Error("dialog unavailable"); };
await assert.rejects(dialogExports.confirmAction("test"), /dialog unavailable/);
FakeElement.prototype.showModal = showModal;
delete globalThis.document;
console.log("Confirmation checks passed: pending, cancel, confirm, Escape, cleanup, and dialog errors.");
