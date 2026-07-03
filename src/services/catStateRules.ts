import type { HardwareMetricsSnapshot } from "../types/hardware";
import type { CatState, CatStateChangedEvent } from "../types/pet";
import type { AppSettings } from "../types/settings";

const minStateHoldMs = 3000;
export const idleSleepAfterMs = 15 * 60 * 1000;
export const fatiguedAfterMs = 90 * 60 * 1000;
export const needsBreakAfterMs = 50 * 60 * 1000;
export const continuousWorkBreakMs = 5 * 60 * 1000;
export const focusNudgeHoldMs = 5 * 60 * 1000;
const focusFatigueProgress = 0.8;
const repairLightLoadThreshold = 76;
const repairHeavyLoadThreshold = 92;
export const temperatureCheckEnterMinCelsius = 75;
export const temperatureCheckExitCelsius = 70;
export const temperatureCheckExitStableMs = 5000;

export interface HardwareCatStateContext {
  cleanupSucceeded?: boolean;
  activeFocusPlannedDurationSeconds?: number | null;
  activeFocusStartedAt?: number | null;
  continuousWorkSince?: number | null;
  focusNudgeState?: Extract<CatState, "NeedsBreak" | "Fatigued"> | null;
  focusNudgeUntil?: number | null;
  lastUserActivityAt?: number;
  now?: Date;
  temperatureSafeSince?: number | null;
  currentCatState?: CatState;
  timestamp?: number;
}

export function deriveCatStatusFromHardware(
  snapshot: HardwareMetricsSnapshot,
  settings: AppSettings,
  context: HardwareCatStateContext = {},
): CatStateChangedEvent {
  const catState = resolveHardwareCatState(snapshot, settings, context);

  return {
    timestamp: context.timestamp ?? Date.now(),
    catState,
    catMessage: messageForCatState(catState),
  };
}

export function resolveHardwareCatState(
  snapshot: HardwareMetricsSnapshot,
  settings: Pick<
    AppSettings,
    | "isCatVisible"
    | "isProductionPaused"
    | "enableSleepMode"
    | "cpuTemperatureWarning"
    | "gpuTemperatureWarning"
    | "memoryCrowdedThreshold"
    | "dataSortingCpuThreshold"
  >,
  context: HardwareCatStateContext = {},
): CatState {
  if (!settings.isCatVisible) {
    return "Hidden";
  }

  if (settings.isProductionPaused) {
    return "Sleep";
  }

  if (isTemperatureWarning(snapshot, settings, context)) {
    return "TemperatureCheck";
  }

  if (
    snapshot.memoryUsagePercent != null &&
    snapshot.memoryUsagePercent > settings.memoryCrowdedThreshold
  ) {
    return "MemoryCrowded";
  }

  if (isHeavyLoad(snapshot)) {
    return "RepairHeavy";
  }

  if (
    snapshot.cpuUsagePercent != null &&
    snapshot.cpuUsagePercent >= settings.dataSortingCpuThreshold
  ) {
    return "RepairLight";
  }

  if (isSustainedBusyLoad(snapshot)) {
    return "RepairLight";
  }

  if (context.cleanupSucceeded) {
    return "Celebrate";
  }

  if (isActiveFocusDistracted(context)) {
    return "Distracted";
  }

  if (isActiveFocusFatigued(context)) {
    return "Fatigued";
  }

  if (isActiveFocusSession(context)) {
    return "DeepWork";
  }

  if (isFocusNudgeActive(context)) {
    return context.focusNudgeState ?? "NeedsBreak";
  }

  if (isInactiveForBreakNudge(context)) {
    return "NeedsBreak";
  }

  if (isFatiguedFromContinuousWork(context)) {
    return "Fatigued";
  }

  if (settings.enableSleepMode && isInactiveForSleep(context)) {
    return "Sleep";
  }

  return "Idle";
}

export function shouldApplyCatStateTransition(
  current: CatState,
  candidate: CatState,
  elapsedMs: number,
) {
  if (current === candidate) {
    return false;
  }

  if (catStateSeverity(candidate) > catStateSeverity(current)) {
    return true;
  }

  return elapsedMs >= minStateHoldMs;
}

export function catStateSeverity(state: CatState) {
  switch (state) {
    case "Hidden":
      return 100;
    case "TemperatureCheck":
      return 90;
    case "RepairHeavy":
      return 80;
    case "MemoryCrowded":
      return 70;
    case "DeepWork":
      return 60;
    case "RepairLight":
      return 50;
    case "NeedsBreak":
      return 46;
    case "Sleep":
      return 40;
    case "Fatigued":
      return 36;
    case "Distracted":
      return 34;
    case "DataSorting":
      return 30;
    case "Celebrate":
      return 15;
    case "Interactive":
      return 8;
    case "Idle":
    default:
      return 0;
  }
}

export function messageForCatState(state: CatState): string {
  switch (state) {
    case "Idle":
      return "CoreCat 正在待命。";
    case "RepairLight":
      return "检测到轻量维护负载。";
    case "RepairHeavy":
      return "系统负载偏高，CoreCat 正在检修。";
    case "TemperatureCheck":
      return "温度偏高，正在关注散热状态。";
    case "MemoryCrowded":
      return "内存较拥挤，建议留意后台任务。";
    case "DataSorting":
      return "系统空闲，CoreCat 正在整理数据。";
    case "Sleep":
      return "长时间未操作，CoreCat 进入休眠。";
    case "Interactive":
      return "CoreCat 正在响应你的操作。";
    case "Celebrate":
      return "清理完成，工坊状态良好。";
    case "Fatigued":
      return "连续工作很久啦，CoreCat 也想歇一会儿。";
    case "NeedsBreak":
      return "久坐提醒：起来活动一下，喝口水吧。";
    case "DeepWork":
      return "专注仪式进行中，CoreCat 陪你一起埋头干活。";
    case "Distracted":
      return "好像走神了？深呼吸，回到任务上来吧。";
    case "Hidden":
      return "";
    default:
      return "";
  }
}

function isTemperatureWarning(
  snapshot: HardwareMetricsSnapshot,
  settings: Pick<AppSettings, "cpuTemperatureWarning" | "gpuTemperatureWarning">,
  context: HardwareCatStateContext,
) {
  if (isTemperatureAboveEnterThreshold(snapshot, settings)) {
    return true;
  }

  if (context.currentCatState !== "TemperatureCheck") {
    return false;
  }

  if (!isTemperatureBelowExitThreshold(snapshot)) {
    return true;
  }

  if (context.temperatureSafeSince == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now - context.temperatureSafeSince < temperatureCheckExitStableMs;
}

export function isTemperatureAboveEnterThreshold(
  snapshot: HardwareMetricsSnapshot,
  settings: Pick<AppSettings, "cpuTemperatureWarning" | "gpuTemperatureWarning">,
) {
  const cpuWarning = Math.max(
    temperatureCheckEnterMinCelsius,
    settings.cpuTemperatureWarning,
  );
  const gpuWarning = Math.max(
    temperatureCheckEnterMinCelsius,
    settings.gpuTemperatureWarning,
  );

  return (
    (snapshot.cpuTemperatureCelsius != null &&
      snapshot.cpuTemperatureCelsius > cpuWarning) ||
    (snapshot.gpuTemperatureCelsius != null &&
      snapshot.gpuTemperatureCelsius > gpuWarning)
  );
}

export function isTemperatureBelowExitThreshold(snapshot: HardwareMetricsSnapshot) {
  const temperatures = [
    snapshot.cpuTemperatureCelsius,
    snapshot.gpuTemperatureCelsius,
  ].filter((value): value is number => value != null && Number.isFinite(value));

  if (temperatures.length === 0) {
    return true;
  }

  return temperatures.every((value) => value < temperatureCheckExitCelsius);
}

function isHeavyLoad(snapshot: HardwareMetricsSnapshot) {
  return (
    (snapshot.cpuUsagePercent != null &&
      snapshot.cpuUsagePercent >= repairHeavyLoadThreshold) ||
    (snapshot.gpuUsagePercent != null &&
      snapshot.gpuUsagePercent >= repairHeavyLoadThreshold)
  );
}

function isSustainedBusyLoad(snapshot: HardwareMetricsSnapshot) {
  return (
    snapshot.gpuUsagePercent != null &&
    snapshot.gpuUsagePercent >= repairLightLoadThreshold
  );
}

function isInactiveForSleep(context: HardwareCatStateContext) {
  if (context.lastUserActivityAt == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now - context.lastUserActivityAt >= idleSleepAfterMs;
}

function isInactiveForBreakNudge(context: HardwareCatStateContext) {
  if (context.lastUserActivityAt == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now - context.lastUserActivityAt >= needsBreakAfterMs;
}

function isActiveFocusSession(context: HardwareCatStateContext) {
  return (
    context.activeFocusStartedAt != null &&
    context.activeFocusPlannedDurationSeconds != null
  );
}

function isActiveFocusDistracted(context: HardwareCatStateContext) {
  if (!isActiveFocusSession(context) || context.lastUserActivityAt == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now - context.lastUserActivityAt >= 90 * 1000;
}

function isActiveFocusFatigued(context: HardwareCatStateContext) {
  if (!isActiveFocusSession(context)) {
    return false;
  }

  const plannedMs = Math.max(
    1,
    (context.activeFocusPlannedDurationSeconds ?? 0) * 1000,
  );
  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return (
    now - (context.activeFocusStartedAt ?? now) >=
    plannedMs * focusFatigueProgress
  );
}

function isFocusNudgeActive(context: HardwareCatStateContext) {
  if (!context.focusNudgeState || context.focusNudgeUntil == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now < context.focusNudgeUntil;
}

function isFatiguedFromContinuousWork(context: HardwareCatStateContext) {
  if (context.continuousWorkSince == null) {
    return false;
  }

  const now = context.timestamp ?? context.now?.getTime() ?? Date.now();
  return now - context.continuousWorkSince >= fatiguedAfterMs;
}
