import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(fileURLToPath(import.meta.url), "..", "..");
const outDir = path.join(repoRoot, ".tmp", "corecat-animation-tests");

rmSync(outDir, { force: true, recursive: true });
mkdirSync(outDir, { recursive: true });

execFileSync(
  process.execPath,
  [path.join(repoRoot, "node_modules", "typescript", "bin", "tsc"), "-p", "tsconfig.corecat-tests.json"],
  {
    cwd: repoRoot,
    stdio: "inherit",
  },
);

writeFileSync(
  path.join(outDir, "package.json"),
  JSON.stringify({ type: "commonjs" }),
);

const require = createRequire(path.join(outDir, "corecat-animation-tests.cjs"));
const {
  CORE_CAT_ANIMATION_CONFIG,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "animationConfig.js",
));
const {
  createCoreCatActionControllerState,
  updateCoreCatActionController,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "animationActionController.js",
));
const {
  getCoreCatOneShotDurationMs,
  getCoreCatTransitionMs,
  resolveCoreCatAnimationState,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "animationStateMachine.js",
));
const { sampleCoreCatPose } = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "animationRuntime.js",
));
const { mixTransitionPose } = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "animationMixer.js",
));
const {
  createCoreCatVfxBus,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "vfx",
  "vfxBus.js",
));
const {
  createCoreCatVfxSnapshot,
  CORECAT_VFX_LIMITS,
  createCoreCatVfxParticleCounts,
  getCoreCatStateVfxEvents,
  shouldPauseHighFrequencyVfx,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "vfx",
  "vfxRuntime.js",
));
const {
  createCoreCatPerformanceMonitor,
  CORECAT_PERFORMANCE_SAMPLE_INTERVAL_MS,
  CORECAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "performance",
  "coreCatPerformanceMonitor.js",
));
const {
  getCoreCatQaSequence,
  getCoreCatQaStepHoldMs,
  shouldCoreCatQaStepAutoFallback,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "debug",
  "coreCatQaSequences.js",
));
const { resolveCoreCatHoverHitArea } = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "hoverHitArea.js",
));
const {
  coreCatSkeletonNodes,
  coreCatSkeletonNodeMap,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "animation",
  "skeletonNodes.js",
));
const {
  coreCatAssetManifest,
  coreCatAssetMetaById,
  coreCatRequiredAssetIds,
  getCoreCatAssetIdForBone,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "assets",
  "coreCatAssetManifest.js",
));
const {
  getCoreCatAssetValidationItem,
  resolveCoreCatAssetRenderMode,
  shouldShowCoreCatAssetPanel,
  validateCoreCatAssets,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "corecat",
  "assets",
  "coreCatAssetValidator.js",
));
const {
  deriveCatStatusFromHardware,
  resolveHardwareCatState,
  shouldApplyCatStateTransition,
} = require(path.join(outDir, "src", "services", "catStateRules.js"));

function input(overrides = {}) {
  return {
    catState: "Idle",
    isClicking: false,
    isDragging: false,
    lowPowerMode: false,
    pointerInside: false,
    staticMode: false,
    ...overrides,
  };
}

assert.equal(resolveCoreCatAnimationState(input()), "idle");
assert.equal(resolveCoreCatAnimationState(input({ pointerInside: true })), "hover");
assert.equal(
  resolveCoreCatAnimationState(input({ isDragging: true, pointerInside: true })),
  "dragging",
);
assert.equal(
  resolveCoreCatAnimationState(input({ isClicking: true, pointerInside: true })),
  "click",
);
assert.equal(
  resolveCoreCatAnimationState(input({ lowPowerMode: true, pointerInside: true })),
  "lowPowerStatic",
);
assert.equal(
  resolveCoreCatAnimationState(input({ isClicking: true, lowPowerMode: true })),
  "click",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({ catState: "TemperatureCheck", lowPowerMode: true, pointerInside: true }),
  ),
  "temperatureCheck",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({ catState: "MemoryCrowded", staticMode: true, pointerInside: true }),
  ),
  "memoryCrowded",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "Sleep", isClicking: true })),
  "click",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({ catState: "Hidden", isClicking: true, pointerInside: true }),
  ),
  "sleep",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({ catState: "TemperatureCheck", isClicking: true, pointerInside: true }),
  ),
  "click",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "MemoryCrowded" })),
  "memoryCrowded",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "RepairHeavy" })),
  "repairing",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "RepairLight" })),
  "repairing",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "DataSorting" })),
  "dataSorting",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "Fatigued" })),
  "fatigued",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "NeedsBreak" })),
  "needsBreak",
);
assert.equal(
  resolveCoreCatAnimationState(input({ catState: "Celebrate" })),
  "celebrate",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "bootWake" })),
  "bootWake",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      completedOneShotState: "bootWake",
      interactionStateOverride: "bootWake",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "dropLanding" })),
  "dropLanding",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      completedOneShotState: "dropLanding",
      interactionStateOverride: "dropLanding",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "panelOpen" })),
  "panelOpen",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "panelClose" })),
  "panelClose",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "pettingHearts" })),
  "pettingHearts",
);
assert.equal(
  resolveCoreCatAnimationState(input({ debugStateOverride: "scaredByMouse" })),
  "scaredByMouse",
);
assert.equal(
  resolveCoreCatAnimationState(input({ debugStateOverride: "eatingFish" })),
  "eatingFish",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "panelOpen",
    }),
  ),
  "panelOpen",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      interactionStateOverride: "panelClose",
      lowPowerMode: true,
    }),
  ),
  "panelClose",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "errorGlitch" })),
  "errorGlitch",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      completedOneShotState: "errorGlitch",
      interactionStateOverride: "errorGlitch",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "updateInstalling" })),
  "updateInstalling",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "achievementPop" })),
  "achievementPop",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "workshopUpgrade" })),
  "workshopUpgrade",
);
assert.equal(
  resolveCoreCatAnimationState(input({ interactionStateOverride: "moduleUpgrade" })),
  "moduleUpgrade",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      completedOneShotState: "pettingHearts",
      interactionStateOverride: "pettingHearts",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "achievementPop",
    }),
  ),
  "achievementPop",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "workshopUpgrade",
    }),
  ),
  "workshopUpgrade",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "MemoryCrowded",
      interactionStateOverride: "moduleUpgrade",
    }),
  ),
  "moduleUpgrade",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "TemperatureCheck",
      completedOneShotState: "workshopUpgrade",
      interactionStateOverride: "workshopUpgrade",
    }),
  ),
  "temperatureCheck",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({
      catState: "MemoryCrowded",
      completedOneShotState: "moduleUpgrade",
      interactionStateOverride: "moduleUpgrade",
    }),
  ),
  "memoryCrowded",
);

assert.equal(
  getCoreCatTransitionMs("idle", "hover"),
  CORE_CAT_ANIMATION_CONFIG.transition.durations.Idle_to_Hover,
);
assert.equal(getCoreCatTransitionMs("hover", "idle"), 180);
assert.equal(getCoreCatTransitionMs("idle", "click"), 40);
assert.equal(getCoreCatTransitionMs("repairing", "celebrate"), 80);
assert.equal(getCoreCatTransitionMs("idle", "workshopUpgrade"), 80);
assert.equal(getCoreCatTransitionMs("workshopUpgrade", "idle"), 240);
assert.equal(getCoreCatTransitionMs("idle", "moduleUpgrade"), 80);
assert.equal(getCoreCatTransitionMs("moduleUpgrade", "idle"), 240);
assert.equal(getCoreCatTransitionMs("idle", "bootWake"), 80);
assert.equal(getCoreCatTransitionMs("bootWake", "idle"), 220);
assert.equal(getCoreCatTransitionMs("idle", "dragging"), 60);
assert.equal(getCoreCatTransitionMs("dragging", "dropLanding"), 40);
assert.equal(getCoreCatTransitionMs("dropLanding", "idle"), 220);
assert.equal(getCoreCatTransitionMs("idle", "panelOpen"), 80);
assert.equal(getCoreCatTransitionMs("panelOpen", "idle"), 160);
assert.equal(getCoreCatTransitionMs("idle", "panelClose"), 80);
assert.equal(getCoreCatTransitionMs("panelClose", "idle"), 160);
assert.equal(getCoreCatTransitionMs("idle", "errorGlitch"), 40);
assert.equal(getCoreCatTransitionMs("errorGlitch", "idle"), 180);
assert.equal(getCoreCatTransitionMs("idle", "updateInstalling"), 160);
assert.equal(getCoreCatTransitionMs("updateInstalling", "celebrate"), 80);
assert.equal(getCoreCatTransitionMs("idle", "achievementPop"), 60);
assert.equal(getCoreCatTransitionMs("achievementPop", "idle"), 160);
assert.equal(getCoreCatTransitionMs("idle", "sleep"), 600);
assert.equal(getCoreCatTransitionMs("sleep", "idle"), 420);
assert.equal(getCoreCatTransitionMs("celebrate", "idle"), 240);
assert.equal(getCoreCatTransitionMs("hover", "hover"), 0);
assert.equal(
  getCoreCatOneShotDurationMs("bootWake"),
  CORE_CAT_ANIMATION_CONFIG.oneShot.bootWakeMs,
);
assert.equal(
  getCoreCatOneShotDurationMs("dropLanding"),
  CORE_CAT_ANIMATION_CONFIG.oneShot.dropLandingMs,
);
assert.equal(getCoreCatOneShotDurationMs("workshopUpgrade"), 5000);
assert.equal(getCoreCatOneShotDurationMs("moduleUpgrade"), 5000);
assert.equal(getCoreCatOneShotDurationMs("updateInstalling"), null);

const fromPose = { body_base: { x: 0, opacity: 0.2 } };
const toPose = { body_base: { x: 10, opacity: 1 } };
assert.equal(mixTransitionPose(fromPose, toPose, 0, 160).body_base.x, 0);
assertNear(mixTransitionPose(fromPose, toPose, 80, 160).body_base.x, 5);
assert.equal(mixTransitionPose(fromPose, toPose, 160, 160).body_base.x, 10);

let actionState = createCoreCatActionControllerState();
let action = updateCoreCatActionController(actionState, true, 1000);
assert.equal(action.didStartClick, true);
assert.equal(action.isClicking, true);

action = updateCoreCatActionController(action.state, true, 1299);
assert.equal(action.didStartClick, false);
assert.equal(action.isClicking, true);

action = updateCoreCatActionController(action.state, true, 1300);
assert.equal(action.didStartClick, false);
assert.equal(action.isClicking, false);
assert.equal(
  resolveCoreCatAnimationState(input({ isClicking: action.isClicking, pointerInside: true })),
  "hover",
);
assert.equal(
  resolveCoreCatAnimationState(
    input({ catState: "Celebrate", completedOneShotState: "celebrate" }),
  ),
  "idle",
);

action = updateCoreCatActionController(action.state, false, 1301);
action = updateCoreCatActionController(action.state, true, 1400);
assert.equal(action.didStartClick, true);
assert.equal(action.isClicking, true);

// Test rapid clicking / click interruption and extension
let rapidAction = createCoreCatActionControllerState();
// First click at 2000ms
rapidAction = updateCoreCatActionController(rapidAction, true, 2000).state;
// Release click at 2080ms
rapidAction = updateCoreCatActionController(rapidAction, false, 2080).state;
// Second click at 2150ms (before 2300ms)
let secondClickRes = updateCoreCatActionController(rapidAction, true, 2150);
assert.equal(secondClickRes.didStartClick, true);
assert.equal(secondClickRes.isClicking, true);
assert.equal(secondClickRes.state.clickUntilMs, 2450);

const hoverRect = { left: 100, top: 50, width: 160, height: 160 };
const hoverCenter = resolveCoreCatHoverHitArea(180, 130, hoverRect);
assert.equal(hoverCenter.isInside, true);
assertNear(hoverCenter.x, 0);
assertNear(hoverCenter.y, 0);
const hoverEdge = resolveCoreCatHoverHitArea(40, 130, hoverRect);
assert.equal(hoverEdge.isInside, true);
assertNear(hoverEdge.x, -1);
const hoverOutside = resolveCoreCatHoverHitArea(39, 130, hoverRect);
assert.equal(hoverOutside.isInside, false);
assert.equal(hoverOutside.x, 0);

assert.deepEqual(
  coreCatSkeletonNodes.map((node) => node.id),
  [
    "root",
    "shadow",
    "tail_base",
    "tail_mid",
    "tail_tip",
    "body_base",
    "arm_left",
    "arm_right_wrench",
    "pouch",
    "head_base",
    "ears_left",
    "ears_right",
    "eyes",
    "goggles",
    "vfx_anchor",
  ],
);
assert.equal(coreCatSkeletonNodeMap.tail_base.parentId, "root");
assert.equal(coreCatSkeletonNodeMap.tail_mid.parentId, "tail_base");
assert.equal(coreCatSkeletonNodeMap.tail_tip.parentId, "tail_mid");
assert.equal(sampleCoreCatPose("idle", {
  blinkActive: false,
  earTwitch: null,
  isClicking: false,
  isDragging: false,
  lowPowerMode: false,
  now: 950,
  pointer: { isInside: false, x: 0, y: 0 },
  reducedMotion: false,
  state: "idle",
  stateElapsedMs: 0,
  staticMode: false,
  updateProgress: 0,
}).tail_tip.rotate !== undefined, true);

const expectedCoreCatAssetIds = [
  "shadow",
  "tail_base",
  "tail_mid",
  "tail_tip",
  "body_base",
  "arm_left",
  "arm_right_wrench",
  "arm_right_fan",
  "pouch",
  "head_base",
  "ears_left",
  "ears_right",
  "goggles",
  "eye_normal",
  "eye_blink",
  "eye_focused",
  "eye_dizzy",
  "eye_sleepy",
  "eye_glowing",
  "ram_box",
  "wrench_clone",
  "badge_star",
  "sleep_bubble",
];
assert.deepEqual(
  coreCatAssetManifest.map((asset) => asset.id),
  expectedCoreCatAssetIds,
);
assert.deepEqual(coreCatRequiredAssetIds, expectedCoreCatAssetIds);
assert.equal(
  coreCatAssetManifest.every((asset) => asset.fallbackPath.length > 0),
  true,
);
assert.equal(getCoreCatAssetIdForBone("body_base", "normal"), "body_base");
assert.equal(getCoreCatAssetIdForBone("eyes", "focused"), "eye_focused");
assert.equal(getCoreCatAssetIdForBone("vfx_anchor", "normal"), null);
assert.equal(coreCatAssetMetaById.body_base.anchorNode, "body_base");
assert.equal(coreCatAssetMetaById.tail_base.anchorNode, "tail_base");
assert.equal(coreCatAssetMetaById.tail_mid.anchorNode, "tail_mid");
assert.equal(coreCatAssetMetaById.tail_tip.anchorNode, "tail_tip");
assert.equal(coreCatAssetMetaById.eye_focused.anchorNode, "eyes");

const emptyAssetReport = validateCoreCatAssets(coreCatAssetManifest, []);
assert.equal(emptyAssetReport.allRequiredSatisfied, false);
assert.equal(emptyAssetReport.formalCount, 0);
assert.equal(emptyAssetReport.placeholderCount, expectedCoreCatAssetIds.length);
assert.equal(
  emptyAssetReport.missingRequired.length,
  expectedCoreCatAssetIds.length,
);
assert.equal(
  getCoreCatAssetValidationItem(emptyAssetReport, "body_base").source,
  "placeholder",
);

const completeAssetReport = validateCoreCatAssets(
  coreCatAssetManifest,
  coreCatAssetManifest.map((asset) => asset.path),
);
assert.equal(completeAssetReport.allRequiredSatisfied, true);
assert.equal(completeAssetReport.formalCount, expectedCoreCatAssetIds.length);
assert.equal(completeAssetReport.placeholderCount, 0);
assert.equal(
  getCoreCatAssetValidationItem(completeAssetReport, "body_base").source,
  "formal",
);
const completeSvgAssetReport = validateCoreCatAssets(
  coreCatAssetManifest,
  coreCatAssetManifest.map((asset) => asset.path.replace(/\.png$/, ".svg")),
);
assert.equal(completeSvgAssetReport.allRequiredSatisfied, true);
assert.equal(completeSvgAssetReport.formalCount, expectedCoreCatAssetIds.length);
assert.equal(
  getCoreCatAssetValidationItem(completeSvgAssetReport, "tail_tip").resolvedPath,
  "corecat_skeleton/tail/tail_tip.svg",
);
assert.equal(resolveCoreCatAssetRenderMode("body_base", null), "placeholder");
assert.equal(resolveCoreCatAssetRenderMode("body_base", "/asset.png"), "formal");
assert.equal(shouldShowCoreCatAssetPanel(false), false);
assert.equal(shouldShowCoreCatAssetPanel(true), true);

const settings = {
  schemaVersion: 1,
  launchAtStartup: false,
  isCatVisible: true,
  isMonitorBarVisible: true,
  enableSleepMode: true,
  enableSound: false,
  enableNotifications: false,
  isProductionPaused: false,
  enableLowPowerMode: false,
  enableStaticCatMode: false,
  enablePetBubble: true,
  showMonitorDataInTaskbar: true,
  samplingIntervalMs: 1000,
  backgroundSamplingIntervalMs: 3000,
  dataSortingCpuThreshold: 40,
  catSize: 1,
  catOpacity: 0.95,
  catWindowX: 0,
  catWindowY: 0,
  monitorBarX: 0,
  monitorBarY: 0,
  cpuTemperatureWarning: 80,
  gpuTemperatureWarning: 82,
  memoryCrowdedThreshold: 82,
  errorGlitchCpuThreshold: 96,
  themeName: "coreworkpal",
  visibleMonitorMetrics: ["Cpu", "Ram", "Network", "Disk"],
  monitorBarMode: "Default",
};
const snapshot = {
  timestamp: 2000,
  cpuUsagePercent: null,
  gpuUsagePercent: null,
  memoryUsagePercent: null,
  cpuTemperatureCelsius: null,
  gpuTemperatureCelsius: null,
  diskReadBytesPerSecond: null,
  diskWriteBytesPerSecond: null,
  networkDownloadBytesPerSecond: null,
  networkUploadBytesPerSecond: null,
  cpuName: null,
  gpuName: null,
  totalMemoryBytes: null,
  usedMemoryBytes: null,
};

assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 80.1 },
    settings,
  ),
  "TemperatureCheck",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, gpuTemperatureCelsius: 82.1 },
    settings,
  ),
  "TemperatureCheck",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, memoryUsagePercent: 82.1 },
    settings,
  ),
  "MemoryCrowded",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuUsagePercent: 93 },
    settings,
  ),
  "RepairHeavy",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, gpuUsagePercent: 77 },
    settings,
  ),
  "RepairLight",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 90 },
    settings,
    { timestamp: 15 * 60 * 1000, lastUserActivityAt: 0 },
  ),
  "TemperatureCheck",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuUsagePercent: 39.9 },
    settings,
  ),
  "Idle",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuUsagePercent: 40 },
    settings,
  ),
  "RepairLight",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuUsagePercent: 16 },
    { ...settings, dataSortingCpuThreshold: 15 },
  ),
  "RepairLight",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 79 },
    { ...settings, cpuTemperatureWarning: 78 },
  ),
  "TemperatureCheck",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 60 },
    { ...settings, cpuTemperatureWarning: 50 },
  ),
  "Idle",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 60 },
    settings,
    {
      currentCatState: "TemperatureCheck",
      temperatureSafeSince: 1000,
      timestamp: 5999,
    },
  ),
  "TemperatureCheck",
);
assert.equal(
  resolveHardwareCatState(
    { ...snapshot, cpuTemperatureCelsius: 60 },
    settings,
    {
      currentCatState: "TemperatureCheck",
      temperatureSafeSince: 1000,
      timestamp: 6000,
    },
  ),
  "Idle",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    { ...settings, isProductionPaused: true },
  ),
  "Sleep",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    { timestamp: 15 * 60 * 1000, lastUserActivityAt: 0 },
  ),
  "Sleep",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    { timestamp: 55 * 60 * 1000, lastUserActivityAt: 0 },
  ),
  "NeedsBreak",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    {
      continuousWorkSince: 0,
      lastUserActivityAt: 100 * 60 * 1000 - 10_000,
      timestamp: 100 * 60 * 1000,
    },
  ),
  "Fatigued",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    {
      activeFocusPlannedDurationSeconds: 25 * 60,
      activeFocusStartedAt: 0,
      lastUserActivityAt: 20 * 60 * 1000 - 10_000,
      timestamp: 20 * 60 * 1000,
    },
  ),
  "Fatigued",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    {
      activeFocusPlannedDurationSeconds: 25 * 60,
      activeFocusStartedAt: 0,
      lastUserActivityAt: 5_000,
      timestamp: 10_000,
    },
  ),
  "DeepWork",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    {
      focusNudgeState: "NeedsBreak",
      focusNudgeUntil: 10_000,
      timestamp: 9_000,
    },
  ),
  "NeedsBreak",
);
assert.equal(
  resolveHardwareCatState(
    snapshot,
    settings,
    {
      focusNudgeState: "NeedsBreak",
      focusNudgeUntil: 10_000,
      timestamp: 10_000,
    },
  ),
  "Idle",
);
assert.equal(
  resolveHardwareCatState(snapshot, settings, { cleanupSucceeded: true }),
  "Celebrate",
);
assert.equal(
  resolveHardwareCatState(snapshot, { ...settings, isCatVisible: false }),
  "Hidden",
);
assert.equal(
  deriveCatStatusFromHardware(
    { ...snapshot, cpuUsagePercent: 40 },
    settings,
    { timestamp: 1234 },
  ).catState,
  "RepairLight",
);
assert.equal(shouldApplyCatStateTransition("RepairLight", "Idle", 1000), false);
assert.equal(shouldApplyCatStateTransition("RepairLight", "Idle", 3000), true);
assert.equal(
  shouldApplyCatStateTransition("Idle", "TemperatureCheck", 0),
  true,
);

const vfxBus = createCoreCatVfxBus();
const receivedVfx = [];
const unsubscribeVfx = vfxBus.subscribe((event) => receivedVfx.push(event));
vfxBus.emit({ intensity: 1, type: "coolingWind" });
vfxBus.emit({ count: 4, type: "spark" });
unsubscribeVfx();
vfxBus.emit({ type: "goldenSteamRing" });
assert.deepEqual(receivedVfx.map((event) => event.type), ["coolingWind", "spark"]);

assert.deepEqual(
  getCoreCatStateVfxEvents("temperatureCheck").map((event) => event.type),
  ["coolingWind", "coldPixels"],
);

const baseVfxInput = {
  animationState: "idle",
  isPaused: false,
  lowPowerMode: false,
  reducedMotion: false,
  sleepBreath: 0.5,
  stars: [],
  stateElapsedMs: 0,
  updateProgress: 0,
};
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "temperatureCheck",
  }).activeEffects.includes("coolingWind"),
);
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "memoryCrowded",
  }).activeEffects.includes("memoryRamBox"),
);
assert.deepEqual(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "sleep",
    stars: [{ delayMs: 0, dx: 1, dy: 1, id: "test" }],
  }).activeEffects,
  ["sleepBubble"],
);
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "celebrate",
    stateElapsedMs: CORE_CAT_ANIMATION_CONFIG.celebrate.burstStartMs,
  }).activeEffects.includes("celebrateBurst"),
);
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "errorGlitch",
  }).activeEffects.includes("errorGlitch"),
);
assert.equal(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "updateInstalling",
    updateProgress: 0.64,
  }).updateProgress,
  0.64,
);
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "updateInstalling",
  }).activeEffects.includes("updateProgress"),
);
assert.ok(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "achievementPop",
  }).activeEffects.includes("achievementBadge"),
);
assert.equal(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "temperatureCheck",
    lowPowerMode: true,
  }).activeEffects.includes("coolingParticles"),
  false,
);
assert.equal(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "sleep",
    lowPowerMode: true,
  }).activeEffects.includes("dataCubes"),
  false,
);
const manyStars = Array.from({ length: 120 }, (_, index) => ({
  delayMs: index,
  dx: index,
  dy: -index,
  id: `many-${index}`,
}));
const cappedVfxSnapshot = createCoreCatVfxSnapshot({
  ...baseVfxInput,
  animationState: "idle",
  stars: manyStars,
});
assert.equal(
  cappedVfxSnapshot.stars.length,
  CORECAT_VFX_LIMITS.maxActiveParticles,
);
assert.equal(
  cappedVfxSnapshot.particleCounts.total <= CORECAT_VFX_LIMITS.maxActiveParticles,
  true,
);
assert.equal(cappedVfxSnapshot.isAutoDegraded, true);
assert.equal(
  createCoreCatVfxParticleCounts(
    ["coolingParticles", "dataCubes", "memorySteam", "repairSparks", "celebrateBurst"],
    0,
  ).coolingParticles <= CORECAT_VFX_LIMITS.maxCoolingParticles,
  true,
);
assert.equal(
  createCoreCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "dataSorting",
    degradeVfx: true,
  }).particleCounts.dataCubes < 8,
  true,
);
assert.equal(shouldPauseHighFrequencyVfx("sleep", false), true);
assert.equal(shouldPauseHighFrequencyVfx("temperatureCheck", false), false);
assert.equal(shouldPauseHighFrequencyVfx("temperatureCheck", true), true);

const qaHardwareSequence = getCoreCatQaSequence("hardware");
assert.ok(getCoreCatQaSequence("all").states.includes("workshopUpgrade"));
assert.ok(getCoreCatQaSequence("all").states.includes("moduleUpgrade"));
assert.deepEqual(qaHardwareSequence.states, [
  "idle",
  "scaredByMouse",
  "eatingFish",
  "memoryCrowded",
  "temperatureCheck",
  "repairing",
  "celebrate",
  "idle",
]);
assert.deepEqual(getCoreCatQaSequence("interaction").states, [
  "bootWake",
  "hover",
  "click",
  "panelOpen",
  "panelClose",
  "pettingHearts",
  "dragging",
  "dropLanding",
  "lowPowerStatic",
  "idle",
]);
assert.equal(shouldCoreCatQaStepAutoFallback("bootWake"), true);
assert.equal(shouldCoreCatQaStepAutoFallback("dropLanding"), true);
assert.equal(shouldCoreCatQaStepAutoFallback("dragging"), false);
assert.equal(
  getCoreCatQaStepHoldMs("bootWake") > CORE_CAT_ANIMATION_CONFIG.oneShot.bootWakeMs,
  true,
);

validateAnimationSpriteSheets();

const perfBaseInput = {
  activeVfxCount: 0,
  animationState: "idle",
  isLowPower: false,
  isVfxAutoDegraded: false,
  isVfxPaused: false,
  previousAnimationState: "idle",
  reducedMotion: false,
  transitionDurationMs: 0,
  vfxParticleCount: 0,
};
const perfMonitor = createCoreCatPerformanceMonitor(perfBaseInput, 0);
let perfReports = 0;
for (let now = 16; now < CORECAT_PERFORMANCE_SAMPLE_INTERVAL_MS; now += 16) {
  if (perfMonitor.recordFrame(perfBaseInput, now)) {
    perfReports += 1;
  }
}
assert.equal(perfReports, 0);
const firstPerfReport = perfMonitor.recordFrame(perfBaseInput, 512);
assert.ok(firstPerfReport);
assert.equal(firstPerfReport.fps > 0, true);
perfMonitor.recordFrame(
  { ...perfBaseInput, animationState: "hover", previousAnimationState: "idle", transitionDurationMs: 120 },
  640,
);
for (let index = 0; index < 12; index += 1) {
  perfMonitor.recordFrame(
    {
      ...perfBaseInput,
      animationState: index % 2 === 0 ? "idle" : "hover",
      previousAnimationState: index % 2 === 0 ? "hover" : "idle",
      transitionDurationMs: 160,
    },
    700 + index * 32,
  );
}
assert.equal(
  perfMonitor.snapshot(1200).stateTransitions.length,
  CORECAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT,
);

console.log("CoreCat animation tests passed");

function assertNear(actual, expected, epsilon = 0.000001) {
  assert.ok(
    Math.abs(actual - expected) <= epsilon,
    `Expected ${actual} to be within ${epsilon} of ${expected}`,
  );
}

function validateAnimationSpriteSheets() {
  const animationDir = path.join(repoRoot, "src", "assets", "pets", "animation");
  const files = readdirSync(animationDir);
  const webpStems = new Set(
    files
      .filter((file) => file.endsWith(".webp"))
      .map((file) => path.basename(file, ".webp")),
  );
  const jsonStems = new Set(
    files
      .filter((file) => file.endsWith(".json"))
      .map((file) => path.basename(file, ".json")),
  );
  const mappedStems = new Set([
    "AchievementPop",
    "BootWake",
    "Celebrate",
    "Click_Action",
    "Click_Dizzy",
    "Dragging",
    "DropLanding",
    "Eating_Fish",
    "ErrorGlitch",
    "Fatigued",
    "Hover",
    "Idle",
    "Memory_Crowded",
    "Module_Upgrade",
    "NeedsBreak",
    "PanelClose",
    "PanelOpen",
    "Petting_Hearts",
    "Repairing",
    "Scared_By_Mouse",
    "Sleep_Low_Power",
    "Temperature_Check",
    "UpdateInstalling",
    "Workshop_Upgrade",
    "dataSorting",
  ]);
  const allStems = new Set([...webpStems, ...jsonStems]);

  assert.deepEqual(
    [...allStems].sort(),
    [...mappedStems].sort(),
    "animation directory should only contain mapped sprite-sheet stems",
  );

  for (const stem of mappedStems) {
    assert.equal(webpStems.has(stem), true, `${stem} should have a WebP sheet`);
    assert.equal(jsonStems.has(stem), true, `${stem} should have JSON metadata`);

    const webp = readFileSync(path.join(animationDir, `${stem}.webp`));
    const { height: webpHeight, width: webpWidth } = readWebpDimensions(webp);
    const meta = JSON.parse(
      readFileSync(path.join(animationDir, `${stem}.json`), "utf8"),
    );

    assert.equal(meta.sheet_size.w, webpWidth, `${stem} sheet width mismatch`);
    assert.equal(meta.sheet_size.h, webpHeight, `${stem} sheet height mismatch`);
    assert.ok(meta.frames.length > 0, `${stem} should have frames`);

    let previousTime = -Infinity;
    for (const [index, frame] of meta.frames.entries()) {
      assert.ok(
        frame.t >= previousTime,
        `${stem} frame ${index} timestamp should be monotonic`,
      );
      previousTime = frame.t;
      assert.ok(
        frame.x >= 0 &&
          frame.y >= 0 &&
          frame.x + meta.frame_size.w <= webpWidth &&
          frame.y + meta.frame_size.h <= webpHeight,
        `${stem} frame ${index} should stay inside the sprite sheet`,
      );
    }
  }
}

function readWebpDimensions(webp) {
  assert.equal(webp.toString("ascii", 0, 4), "RIFF", "invalid WebP RIFF header");
  assert.equal(webp.toString("ascii", 8, 12), "WEBP", "invalid WebP signature");

  let offset = 12;
  while (offset + 8 <= webp.length) {
    const chunkType = webp.toString("ascii", offset, offset + 4);
    const chunkSize = webp.readUInt32LE(offset + 4);
    const chunkOffset = offset + 8;

    if (chunkType === "VP8X") {
      return {
        width: 1 + webp.readUIntLE(chunkOffset + 4, 3),
        height: 1 + webp.readUIntLE(chunkOffset + 7, 3),
      };
    }

    if (chunkType === "VP8 ") {
      return {
        width: webp.readUInt16LE(chunkOffset + 6) & 0x3fff,
        height: webp.readUInt16LE(chunkOffset + 8) & 0x3fff,
      };
    }

    if (chunkType === "VP8L") {
      const b0 = webp[chunkOffset + 1];
      const b1 = webp[chunkOffset + 2];
      const b2 = webp[chunkOffset + 3];
      const b3 = webp[chunkOffset + 4];

      return {
        width: 1 + (((b1 & 0x3f) << 8) | b0),
        height: 1 + (((b3 & 0x0f) << 10) | (b2 << 2) | ((b1 & 0xc0) >> 6)),
      };
    }

    offset = chunkOffset + chunkSize + (chunkSize % 2);
  }

  throw new Error("unsupported WebP file: missing VP8/VP8L/VP8X chunk");
}
