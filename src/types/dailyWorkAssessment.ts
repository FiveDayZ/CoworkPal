import type { WorkLogScoreDimension } from "./workLog";

export type WorkDayType =
  | "deepFocus"
  | "buildBurst"
  | "archiveFlow"
  | "pressureRepair"
  | "stableMaintenance"
  | "fragmentedSwitching"
  | "lowLoadCompanion"
  | "unknown";

export type InsightSeverity = "positive" | "neutral" | "warning";

export type CoreCatCommentTone =
  | "encouragement"
  | "tease"
  | "warning"
  | "celebration";

export type WorkTimelineSegmentKind =
  | "idleCompanion"
  | "steadyProgress"
  | "deepFocus"
  | "buildPeak"
  | "archiveFlow"
  | "memoryCrowded"
  | "temperatureWarning"
  | "pressureRepair";

export interface WorkprintSummary {
  label: string;
  description: string;
  pixelGrid: number[];
  width: number;
  height: number;
  loadShape: number;
  inputRhythm: number;
  ioIntensity: number;
  thermalPressure: number;
  continuity: number;
}

export interface WorkTimelineSegment {
  startTime: string;
  endTime: string;
  kind: WorkTimelineSegmentKind;
  intensity: number;
  label: string;
  description: string;
}

export interface BaselineComparison {
  sampleDays: number;
  activeSecondsDeltaRatio: number;
  loadDeltaRatio: number;
  ioDeltaRatio: number;
  thermalDeltaRatio: number;
  inputDeltaRatio: number;
  metrics: BaselineMetric[];
  summary: string;
}

export interface BaselineMetric {
  key: string;
  label: string;
  currentValue: string;
  baselineValue: string;
  deltaRatio: number;
  tone: InsightSeverity;
}

export interface AssessmentInsight {
  title: string;
  body: string;
  severity: InsightSeverity;
  metricValue?: string | null;
}

export interface ProcessUsageInsight {
  name: string;
  observedSeconds: number;
  activeSeconds: number;
  sampleCount: number;
  activeSampleCount: number;
  cpuPressurePercent: number;
  averageMemoryBytes: number;
  memoryBytesPeak: number;
  diskReadBytesTotal: number;
  diskWriteBytesTotal: number;
  rankLabel: string;
  summary: string;
  severity: InsightSeverity;
}

export interface WorkCardRarity {
  tier: "C" | "B" | "A" | "S" | "SS" | string;
  label: string;
  score: number;
  reason: string;
}

export interface WorkDayTitle {
  family: string;
  title: string;
  level: number;
  progress: number;
  nextLevelAt?: number | null;
}

export interface CoreCatCommentary {
  tone: CoreCatCommentTone;
  title: string;
  body: string;
}

export interface DailyWorkAssessment {
  date: string;
  dayType: WorkDayType;
  dayTypeTitle: string;
  rarity: WorkCardRarity;
  title: WorkDayTitle;
  corecatCommentary: CoreCatCommentary;
  workprint: WorkprintSummary;
  baseline: BaselineComparison;
  timeline: WorkTimelineSegment[];
  mvpSegments: WorkTimelineSegment[];
  highlights: AssessmentInsight[];
  risks: AssessmentInsight[];
  suggestions: AssessmentInsight[];
  processInsights: ProcessUsageInsight[];
  dimensions: WorkLogScoreDimension[];
  score: number;
  corecatSummary: string;
  badgeIds: string[];
}

export interface DailyWorkAssessmentSummary {
  date: string;
  dayType: WorkDayType;
  dayTypeTitle: string;
  rarity: WorkCardRarity;
  title: WorkDayTitle;
  workprint: WorkprintSummary;
  score: number;
  corecatSummary: string;
  badgeIds: string[];
  hasTimeline: boolean;
  hasData: boolean;
}

export interface DailyWorkAssessmentTrend {
  sampleDays: number;
  averageScore: number;
  bestDate: string | null;
  bestScore: number | null;
  bestDayType: WorkDayType;
  bestDayTypeTitle: string;
  dominantDayType: WorkDayType;
  dominantDayTypeTitle: string;
  timelineDays: number;
  scoreDelta: number;
  summary: string;
  insights: AssessmentInsight[];
}
