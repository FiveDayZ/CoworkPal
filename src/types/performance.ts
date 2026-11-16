import type { ProcessUsageSnapshot } from "./hardware";
export interface PerformanceSample { timestamp: number; values: Array<number | null>; }
export interface PerformanceEvent { metric: number; startedAt: number; endedAt: number | null; lastSeenAt: number; interrupted: boolean; peak: number; threshold: number; processes: ProcessUsageSnapshot[]; }
export interface PerformanceHistory { samples: PerformanceSample[]; events: PerformanceEvent[]; }
