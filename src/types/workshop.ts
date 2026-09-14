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
