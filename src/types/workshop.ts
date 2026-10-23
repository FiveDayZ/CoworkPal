import type { RewardReceipt } from "./rewards";

export const MAX_AFFINITY_LEVEL = 100;

export function workshopLevelSteps(level: number): number {
  const capped = Math.max(1, Math.min(255, level));
  return Math.min(capped - 1, 99) + Math.max(0, Math.min(capped - 100, 100)) * 0.75
    + Math.max(0, capped - 200) * 0.5;
}

export function workshopCoolingRelief(levels: ModuleUpgradeLevels): number {
  const bonus = workshopLevelSteps(levels.parts) * 0.003 + workshopLevelSteps(levels.process) * 0.0055;
  return bonus <= 0.29 ? bonus : 0.58 - 0.29 * Math.exp(-(bonus - 0.29) / 0.29);
}

export function addAffinityExperience(workshop: WorkshopState, amount: number): void {
  const experience = workshop.affinityExperience + amount;
  workshop.catAffinityLevel = Math.min(MAX_AFFINITY_LEVEL,
    Math.max(1, workshop.catAffinityLevel) + Math.floor(experience / 100));
  workshop.affinityExperience = workshop.catAffinityLevel === MAX_AFFINITY_LEVEL ? 0 : experience % 100;
}

export type WorkshopModuleKey =
  | "cpu"
  | "gpu"
  | "ram"
  | "network"
  | "temperature"
  | "disk";

export interface ModuleUpgradeLevels {
  parts: number;
  process: number;
}

export interface WorkshopModuleLevels {
  cpu: ModuleUpgradeLevels;
  gpu: ModuleUpgradeLevels;
  ram: ModuleUpgradeLevels;
  network: ModuleUpgradeLevels;
  temperature: ModuleUpgradeLevels;
  disk: ModuleUpgradeLevels;
}

export interface WorkshopState {
  schemaVersion: number;
  parts: number;
  insight: number;
  workshopLevel: number;
  catAffinityLevel: number;
  moduleLevels: WorkshopModuleLevels;
  lastProductionTime: number;
  totalOnlineSeconds: number;
  todayParts: number;
  todayInsight: number;
  lastDailyResetDate: string;
  affinityExperience: number;
  completedOrderCount: number;
  activeOrders: WorkshopOrder[];
  completedOrderIds: string[];
  lastOrderRefreshDate: string;
  rewardReceipts: Record<string, RewardReceipt>;
}

export type WorkshopOrderKind = "standard" | "timed" | "hardwareEvent";

export interface WorkshopOrder {
  id: string;
  kind: WorkshopOrderKind;
  title: string;
  description: string;
  requiredParts: number;
  requiredInsight: number;
  rewardAffinity: number;
  createdAt: number;
  expiresAt: number | null;
}

export interface WorkshopProductionBreakdown {
  partsPerMinute: number;
  insightPerMinute: number;
  partsActivity: number;
  insightActivity: number;
  workshopMultiplier: number;
  partsModuleMultiplier: number;
  insightModuleMultiplier: number;
  stabilityMultiplier: number;
  focusMultiplier: number;
  affinityMultiplier: number;
  affinityTitle: string;
  affinityTier: "new" | "trusted" | "partner" | "bonded";
  nextAffinityLevel: number | null;
  nextAffinityTitle: string | null;
}

export interface ResourceCost {
  parts: number;
  insight: number;
}

export interface ModuleUpgradeCosts {
  parts: ResourceCost | null;
  process: ResourceCost | null;
}

export interface WorkshopUpgradeQuotes {
  workshop: ResourceCost | null;
  modules: Record<WorkshopModuleKey, ModuleUpgradeCosts>;
}

export const defaultModuleLevels: WorkshopModuleLevels = {
  cpu: { parts: 1, process: 1 },
  gpu: { parts: 1, process: 1 },
  ram: { parts: 1, process: 1 },
  network: { parts: 1, process: 1 },
  temperature: { parts: 1, process: 1 },
  disk: { parts: 1, process: 1 },
};

export function normalizeModuleLevels(
  levels: Partial<WorkshopModuleLevels> | null | undefined,
): WorkshopModuleLevels {
  return {
    cpu: normalizeModuleLevel(levels?.cpu),
    gpu: normalizeModuleLevel(levels?.gpu),
    ram: normalizeModuleLevel(levels?.ram),
    network: normalizeModuleLevel(levels?.network),
    temperature: normalizeModuleLevel(levels?.temperature),
    disk: normalizeModuleLevel(levels?.disk),
  };
}

function normalizeModuleLevel(
  level: Partial<ModuleUpgradeLevels> | null | undefined,
): ModuleUpgradeLevels {
  return {
    parts: Math.max(1, Math.floor(level?.parts ?? 1)),
    process: Math.max(1, Math.floor(level?.process ?? 1)),
  };
}
