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
  "cocat",
  "animation",
  "animationConfig.js",
));
const {
  createCoCatActionControllerState,
  updateCoCatActionController,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "animationActionController.js",
));
const {
  getCoCatOneShotDurationMs,
  getCoCatTransitionMs,
  resolveCoCatAnimationState,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "animationStateMachine.js",
));
const { sampleCoCatPose } = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "animationRuntime.js",
));
const { mixTransitionPose } = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "animationMixer.js",
));
const {
  createCoCatVfxBus,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "vfx",
  "vfxBus.js",
));
const {
  createCoCatVfxSnapshot,
  COCAT_VFX_LIMITS,
  createCoCatVfxParticleCounts,
  getCoCatStateVfxEvents,
  shouldPauseHighFrequencyVfx,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "vfx",
  "vfxRuntime.js",
));
const {
  createCoCatPerformanceMonitor,
  COCAT_PERFORMANCE_SAMPLE_INTERVAL_MS,
  COCAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "performance",
  "coCatPerformanceMonitor.js",
));
const {
  getCoCatQaSequence,
  getCoCatQaStepHoldMs,
  shouldCoCatQaStepAutoFallback,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "debug",
  "coCatQaSequences.js",
));
const { resolveCoCatHoverHitArea } = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "hoverHitArea.js",
));
const {
  coCatSkeletonNodes,
  coCatSkeletonNodeMap,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "animation",
  "skeletonNodes.js",
));
const {
  coCatAssetManifest,
  coCatAssetMetaById,
  coCatRequiredAssetIds,
  getCoCatAssetIdForBone,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "assets",
  "coCatAssetManifest.js",
));
const {
  getCoCatAssetValidationItem,
  resolveCoCatAssetRenderMode,
  shouldShowCoCatAssetPanel,
  validateCoCatAssets,
} = require(path.join(
  outDir,
  "src",
  "pet",
  "cocat",
  "assets",
  "coCatAssetValidator.js",
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

assert.equal(resolveCoCatAnimationState(input()), "idle");
assert.equal(resolveCoCatAnimationState(input({ pointerInside: true })), "hover");
assert.equal(
  resolveCoCatAnimationState(input({ isDragging: true, pointerInside: true })),
  "dragging",
);
assert.equal(
  resolveCoCatAnimationState(input({ isClicking: true, pointerInside: true })),
  "click",
);
assert.equal(
  resolveCoCatAnimationState(input({ lowPowerMode: true, pointerInside: true })),
  "lowPowerStatic",
);
assert.equal(
  resolveCoCatAnimationState(input({ isClicking: true, lowPowerMode: true })),
  "click",
);
assert.equal(
  resolveCoCatAnimationState(
    input({ catState: "TemperatureCheck", lowPowerMode: true, pointerInside: true }),
  ),
  "temperatureCheck",
);
assert.equal(
  resolveCoCatAnimationState(
    input({ catState: "MemoryCrowded", staticMode: true, pointerInside: true }),
  ),
  "memoryCrowded",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "Sleep", isClicking: true })),
  "click",
);
assert.equal(
  resolveCoCatAnimationState(
    input({ catState: "Hidden", isClicking: true, pointerInside: true }),
  ),
  "sleep",
);
assert.equal(
  resolveCoCatAnimationState(
    input({ catState: "TemperatureCheck", isClicking: true, pointerInside: true }),
  ),
  "click",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "MemoryCrowded" })),
  "memoryCrowded",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "RepairHeavy" })),
  "repairing",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "RepairLight" })),
  "repairing",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "DataSorting" })),
  "dataSorting",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "Fatigued" })),
  "fatigued",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "NeedsBreak" })),
  "needsBreak",
);
assert.equal(
  resolveCoCatAnimationState(input({ catState: "Celebrate" })),
  "celebrate",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "bootWake" })),
  "bootWake",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      completedOneShotState: "bootWake",
      interactionStateOverride: "bootWake",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "dropLanding" })),
  "dropLanding",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      completedOneShotState: "dropLanding",
      interactionStateOverride: "dropLanding",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "panelOpen" })),
  "panelOpen",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "panelClose" })),
  "panelClose",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "pettingHearts" })),
  "pettingHearts",
);
assert.equal(
  resolveCoCatAnimationState(input({ debugStateOverride: "scaredByMouse" })),
  "scaredByMouse",
);
assert.equal(
  resolveCoCatAnimationState(input({ debugStateOverride: "eatingFish" })),
  "eatingFish",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "panelOpen",
    }),
  ),
  "panelOpen",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      interactionStateOverride: "panelClose",
      lowPowerMode: true,
    }),
  ),
  "panelClose",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "errorGlitch" })),
  "errorGlitch",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      completedOneShotState: "errorGlitch",
      interactionStateOverride: "errorGlitch",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "updateInstalling" })),
  "updateInstalling",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "achievementPop" })),
  "achievementPop",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "workshopUpgrade" })),
  "workshopUpgrade",
);
assert.equal(
  resolveCoCatAnimationState(input({ interactionStateOverride: "moduleUpgrade" })),
  "moduleUpgrade",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      completedOneShotState: "pettingHearts",
      interactionStateOverride: "pettingHearts",
    }),
  ),
  "idle",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "achievementPop",
    }),
  ),
  "achievementPop",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "TemperatureCheck",
      interactionStateOverride: "workshopUpgrade",
    }),
  ),
  "workshopUpgrade",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "MemoryCrowded",
      interactionStateOverride: "moduleUpgrade",
    }),
  ),
  "moduleUpgrade",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "TemperatureCheck",
      completedOneShotState: "workshopUpgrade",
      interactionStateOverride: "workshopUpgrade",
    }),
  ),
  "temperatureCheck",
);
assert.equal(
  resolveCoCatAnimationState(
    input({
      catState: "MemoryCrowded",
      completedOneShotState: "moduleUpgrade",
      interactionStateOverride: "moduleUpgrade",
    }),
  ),
  "memoryCrowded",
);

assert.equal(
  getCoCatTransitionMs("idle", "hover"),
  CORE_CAT_ANIMATION_CONFIG.transition.durations.Idle_to_Hover,
);
assert.equal(getCoCatTransitionMs("hover", "idle"), 180);
assert.equal(getCoCatTransitionMs("idle", "click"), 40);
assert.equal(getCoCatTransitionMs("repairing", "celebrate"), 80);
assert.equal(getCoCatTransitionMs("idle", "workshopUpgrade"), 80);
assert.equal(getCoCatTransitionMs("workshopUpgrade", "idle"), 240);
assert.equal(getCoCatTransitionMs("idle", "moduleUpgrade"), 80);
assert.equal(getCoCatTransitionMs("moduleUpgrade", "idle"), 240);
assert.equal(getCoCatTransitionMs("idle", "bootWake"), 80);
assert.equal(getCoCatTransitionMs("bootWake", "idle"), 220);
assert.equal(getCoCatTransitionMs("idle", "dragging"), 60);
assert.equal(getCoCatTransitionMs("dragging", "dropLanding"), 40);
assert.equal(getCoCatTransitionMs("dropLanding", "idle"), 220);
assert.equal(getCoCatTransitionMs("idle", "panelOpen"), 80);
assert.equal(getCoCatTransitionMs("panelOpen", "idle"), 160);
assert.equal(getCoCatTransitionMs("idle", "panelClose"), 80);
assert.equal(getCoCatTransitionMs("panelClose", "idle"), 160);
assert.equal(getCoCatTransitionMs("idle", "errorGlitch"), 40);
assert.equal(getCoCatTransitionMs("errorGlitch", "idle"), 180);
assert.equal(getCoCatTransitionMs("idle", "updateInstalling"), 160);
assert.equal(getCoCatTransitionMs("updateInstalling", "celebrate"), 80);
assert.equal(getCoCatTransitionMs("idle", "achievementPop"), 60);
assert.equal(getCoCatTransitionMs("achievementPop", "idle"), 160);
assert.equal(getCoCatTransitionMs("idle", "sleep"), 600);
assert.equal(getCoCatTransitionMs("sleep", "idle"), 420);
assert.equal(getCoCatTransitionMs("celebrate", "idle"), 240);
assert.equal(getCoCatTransitionMs("hover", "hover"), 0);
assert.equal(
  getCoCatOneShotDurationMs("bootWake"),
  CORE_CAT_ANIMATION_CONFIG.oneShot.bootWakeMs,
);
assert.equal(
  getCoCatOneShotDurationMs("dropLanding"),
  CORE_CAT_ANIMATION_CONFIG.oneShot.dropLandingMs,
);
assert.equal(getCoCatOneShotDurationMs("workshopUpgrade"), 5000);
assert.equal(getCoCatOneShotDurationMs("moduleUpgrade"), 5000);
assert.equal(getCoCatOneShotDurationMs("updateInstalling"), null);

const fromPose = { body_base: { x: 0, opacity: 0.2 } };
const toPose = { body_base: { x: 10, opacity: 1 } };
assert.equal(mixTransitionPose(fromPose, toPose, 0, 160).body_base.x, 0);
assertNear(mixTransitionPose(fromPose, toPose, 80, 160).body_base.x, 5);
assert.equal(mixTransitionPose(fromPose, toPose, 160, 160).body_base.x, 10);

let actionState = createCoCatActionControllerState();
let action = updateCoCatActionController(actionState, true, 1000);
assert.equal(action.didStartClick, true);
assert.equal(action.isClicking, true);

action = updateCoCatActionController(action.state, true, 1299);
assert.equal(action.didStartClick, false);
assert.equal(action.isClicking, true);

action = updateCoCatActionController(action.state, true, 1300);
assert.equal(action.didStartClick, false);
assert.equal(action.isClicking, false);
assert.equal(
  resolveCoCatAnimationState(input({ isClicking: action.isClicking, pointerInside: true })),
  "hover",
);
assert.equal(
  resolveCoCatAnimationState(
    input({ catState: "Celebrate", completedOneShotState: "celebrate" }),
  ),
  "idle",
);

action = updateCoCatActionController(action.state, false, 1301);
action = updateCoCatActionController(action.state, true, 1400);
assert.equal(action.didStartClick, true);
assert.equal(action.isClicking, true);

// Test rapid clicking / click interruption and extension
let rapidAction = createCoCatActionControllerState();
// First click at 2000ms
rapidAction = updateCoCatActionController(rapidAction, true, 2000).state;
// Release click at 2080ms
rapidAction = updateCoCatActionController(rapidAction, false, 2080).state;
// Second click at 2150ms (before 2300ms)
let secondClickRes = updateCoCatActionController(rapidAction, true, 2150);
assert.equal(secondClickRes.didStartClick, true);
assert.equal(secondClickRes.isClicking, true);
assert.equal(secondClickRes.state.clickUntilMs, 2450);

const hoverRect = { left: 100, top: 50, width: 160, height: 160 };
const hoverCenter = resolveCoCatHoverHitArea(180, 130, hoverRect);
assert.equal(hoverCenter.isInside, true);
assertNear(hoverCenter.x, 0);
assertNear(hoverCenter.y, 0);
const hoverEdge = resolveCoCatHoverHitArea(40, 130, hoverRect);
assert.equal(hoverEdge.isInside, true);
assertNear(hoverEdge.x, -1);
const hoverOutside = resolveCoCatHoverHitArea(39, 130, hoverRect);
assert.equal(hoverOutside.isInside, false);
assert.equal(hoverOutside.x, 0);

assert.deepEqual(
  coCatSkeletonNodes.map((node) => node.id),
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
assert.equal(coCatSkeletonNodeMap.tail_base.parentId, "root");
assert.equal(coCatSkeletonNodeMap.tail_mid.parentId, "tail_base");
assert.equal(coCatSkeletonNodeMap.tail_tip.parentId, "tail_mid");
assert.equal(sampleCoCatPose("idle", {
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

const expectedCoCatAssetIds = [
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
  coCatAssetManifest.map((asset) => asset.id),
  expectedCoCatAssetIds,
);
assert.deepEqual(coCatRequiredAssetIds, expectedCoCatAssetIds);
assert.equal(
  coCatAssetManifest.every((asset) => asset.fallbackPath.length > 0),
  true,
);
assert.equal(getCoCatAssetIdForBone("body_base", "normal"), "body_base");
assert.equal(getCoCatAssetIdForBone("eyes", "focused"), "eye_focused");
assert.equal(getCoCatAssetIdForBone("vfx_anchor", "normal"), null);
assert.equal(coCatAssetMetaById.body_base.anchorNode, "body_base");
assert.equal(coCatAssetMetaById.tail_base.anchorNode, "tail_base");
assert.equal(coCatAssetMetaById.tail_mid.anchorNode, "tail_mid");
assert.equal(coCatAssetMetaById.tail_tip.anchorNode, "tail_tip");
assert.equal(coCatAssetMetaById.eye_focused.anchorNode, "eyes");

const emptyAssetReport = validateCoCatAssets(coCatAssetManifest, []);
assert.equal(emptyAssetReport.allRequiredSatisfied, false);
assert.equal(emptyAssetReport.formalCount, 0);
assert.equal(emptyAssetReport.placeholderCount, expectedCoCatAssetIds.length);
assert.equal(
  emptyAssetReport.missingRequired.length,
  expectedCoCatAssetIds.length,
);
assert.equal(
  getCoCatAssetValidationItem(emptyAssetReport, "body_base").source,
  "placeholder",
);

const completeAssetReport = validateCoCatAssets(
  coCatAssetManifest,
  coCatAssetManifest.map((asset) => asset.path),
);
assert.equal(completeAssetReport.allRequiredSatisfied, true);
assert.equal(completeAssetReport.formalCount, expectedCoCatAssetIds.length);
assert.equal(completeAssetReport.placeholderCount, 0);
assert.equal(
  getCoCatAssetValidationItem(completeAssetReport, "body_base").source,
  "formal",
);
const completeSvgAssetReport = validateCoCatAssets(
  coCatAssetManifest,
  coCatAssetManifest.map((asset) => asset.path.replace(/\.png$/, ".svg")),
);
assert.equal(completeSvgAssetReport.allRequiredSatisfied, true);
assert.equal(completeSvgAssetReport.formalCount, expectedCoCatAssetIds.length);
assert.equal(
  getCoCatAssetValidationItem(completeSvgAssetReport, "tail_tip").resolvedPath,
  "cocat_skeleton/tail/tail_tip.svg",
);
assert.equal(resolveCoCatAssetRenderMode("body_base", null), "placeholder");
assert.equal(resolveCoCatAssetRenderMode("body_base", "/asset.png"), "formal");
assert.equal(shouldShowCoCatAssetPanel(false), false);
assert.equal(shouldShowCoCatAssetPanel(true), true);

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
  themeName: "coworkpal",
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

const vfxBus = createCoCatVfxBus();
const receivedVfx = [];
const unsubscribeVfx = vfxBus.subscribe((event) => receivedVfx.push(event));
vfxBus.emit({ intensity: 1, type: "coolingWind" });
vfxBus.emit({ count: 4, type: "spark" });
unsubscribeVfx();
vfxBus.emit({ type: "goldenSteamRing" });
assert.deepEqual(receivedVfx.map((event) => event.type), ["coolingWind", "spark"]);

assert.deepEqual(
  getCoCatStateVfxEvents("temperatureCheck").map((event) => event.type),
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
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "temperatureCheck",
  }).activeEffects.includes("coolingWind"),
);
assert.ok(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "memoryCrowded",
  }).activeEffects.includes("memoryRamBox"),
);
assert.deepEqual(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "sleep",
    stars: [{ delayMs: 0, dx: 1, dy: 1, id: "test" }],
  }).activeEffects,
  ["sleepBubble"],
);
assert.ok(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "celebrate",
    stateElapsedMs: CORE_CAT_ANIMATION_CONFIG.celebrate.burstStartMs,
  }).activeEffects.includes("celebrateBurst"),
);
assert.ok(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "errorGlitch",
  }).activeEffects.includes("errorGlitch"),
);
assert.equal(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "updateInstalling",
    updateProgress: 0.64,
  }).updateProgress,
  0.64,
);
assert.ok(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "updateInstalling",
  }).activeEffects.includes("updateProgress"),
);
assert.ok(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "achievementPop",
  }).activeEffects.includes("achievementBadge"),
);
assert.equal(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "temperatureCheck",
    lowPowerMode: true,
  }).activeEffects.includes("coolingParticles"),
  false,
);
assert.equal(
  createCoCatVfxSnapshot({
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
const cappedVfxSnapshot = createCoCatVfxSnapshot({
  ...baseVfxInput,
  animationState: "idle",
  stars: manyStars,
});
assert.equal(
  cappedVfxSnapshot.stars.length,
  COCAT_VFX_LIMITS.maxActiveParticles,
);
assert.equal(
  cappedVfxSnapshot.particleCounts.total <= COCAT_VFX_LIMITS.maxActiveParticles,
  true,
);
assert.equal(cappedVfxSnapshot.isAutoDegraded, true);
assert.equal(
  createCoCatVfxParticleCounts(
    ["coolingParticles", "dataCubes", "memorySteam", "repairSparks", "celebrateBurst"],
    0,
  ).coolingParticles <= COCAT_VFX_LIMITS.maxCoolingParticles,
  true,
);
assert.equal(
  createCoCatVfxSnapshot({
    ...baseVfxInput,
    animationState: "dataSorting",
    degradeVfx: true,
  }).particleCounts.dataCubes < 8,
  true,
);
assert.equal(shouldPauseHighFrequencyVfx("sleep", false), true);
assert.equal(shouldPauseHighFrequencyVfx("temperatureCheck", false), false);
assert.equal(shouldPauseHighFrequencyVfx("temperatureCheck", true), true);

const qaHardwareSequence = getCoCatQaSequence("hardware");
assert.ok(getCoCatQaSequence("all").states.includes("workshopUpgrade"));
assert.ok(getCoCatQaSequence("all").states.includes("moduleUpgrade"));
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
assert.deepEqual(getCoCatQaSequence("interaction").states, [
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
assert.equal(shouldCoCatQaStepAutoFallback("bootWake"), true);
assert.equal(shouldCoCatQaStepAutoFallback("dropLanding"), true);
assert.equal(shouldCoCatQaStepAutoFallback("dragging"), false);
assert.equal(
  getCoCatQaStepHoldMs("bootWake") > CORE_CAT_ANIMATION_CONFIG.oneShot.bootWakeMs,
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
const perfMonitor = createCoCatPerformanceMonitor(perfBaseInput, 0);
let perfReports = 0;
for (let now = 16; now < COCAT_PERFORMANCE_SAMPLE_INTERVAL_MS; now += 16) {
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
  COCAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT,
);

console.log("CoCat animation tests passed");

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
    "Free_Memory",
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
