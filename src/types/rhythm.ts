export interface RhythmBucket {
  index: number;
  activeSeconds: number;
  avgFocusScore: number;
  sampleDays: number;
}

export interface RhythmProfile {
  hourBuckets: RhythmBucket[];
  weekdayBuckets: RhythmBucket[];
  peakHours: number[];
  summary: string;
  bestWorkWindow: string | null;
  lowEnergyWindow: string | null;
  interruptionSource: string;
  sampleDays: number;
  confidence: string;
}
