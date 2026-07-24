/**
 * Multi-day health-trend report types — mirror of the Rust `HealthTrendReport`
 * family in src-tauri/src/models.rs, returned by the `get_health_trend` command.
 */

export type TrendRange = "days7" | "days30" | "days90";

/** Frontend range selector key → the string the Rust command expects ("7"|"30"|"90"). */
export const TREND_RANGE_VALUES: Record<TrendRange, string> = {
  days7: "7",
  days30: "30",
  days90: "90",
};

export interface TrendDayPoint {
  date: string;
  totalScore: number;
  durationScore: number;
  loadScore: number;
  complexityScore: number;
  stabilityScore: number;
  continuityScore: number;
  activeSeconds: number;
  hasData: boolean;
}

export interface TrendAverages {
  score: number;
  activeHours: number;
  cpuAvg: number;
  memoryAvg: number;
  thermalAvg: number;
  highLoadRatio: number;
}

export interface TrendPeaks {
  bestScoreDate: string | null;
  bestScore: number | null;
  longestDayDate: string | null;
  longestHours: number | null;
  hottestDayDate: string | null;
  hottestThermal: number | null;
}

export interface TrendWeekdayStat {
  weekday: number; // 0 = Monday .. 6 = Sunday
  avgScore: number;
  avgHours: number;
  sampleDays: number;
}

export interface TrendStreaks {
  current: number;
  longest: number;
  totalActiveDays: number;
}

export interface TrendDelta {
  scoreDelta: number;
  hoursDelta: number;
  tone: "positive" | "neutral" | "warning" | string;
}

export interface HealthTrendReport {
  windowDays: number;
  range: TrendRange;
  scoreSeries: TrendDayPoint[];
  averages: TrendAverages;
  peaks: TrendPeaks;
  weekdayBreakdown: TrendWeekdayStat[];
  streaks: TrendStreaks;
  deltaVsPrev: TrendDelta;
  healthScore: number;
  healthGrade: string;
  summary: string;
}
