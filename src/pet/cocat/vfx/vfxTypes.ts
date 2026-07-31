import type { CoCatAnimationState, CoCatStarParticle } from "../animation/animationTypes";

export type CoCatVfxEvent =
  | { type: "spark"; count?: number }
  | { type: "clickStars"; count?: number }
  | { type: "coolingWind"; intensity: number }
  | { type: "coldPixels"; intensity: number }
  | { type: "steamBurst"; count?: number }
  | { type: "dataCubeSpawn"; count?: number }
  | { type: "errorGlitch" }
  | { type: "pouchGlow" }
  | { type: "sleepBubblePulse" }
  | { type: "goldenSteamRing" }
  | { type: "achievementPop" }
  | { type: "updateProgress"; progress?: number }
  | { type: "goggleShimmer" };

export type CoCatVfxEffectId =
  | "clickStars"
  | "coolingWind"
  | "coolingParticles"
  | "coolingText"
  | "memoryRamBox"
  | "memorySteam"
  | "repairSparks"
  | "hologramPanel"
  | "dataCubes"
  | "celebrateBurst"
  | "sleepBubble"
  | "errorGlitch"
  | "updateProgress"
  | "achievementBadge";

export interface CoCatVfxRuntimeInput {
  animationState: CoCatAnimationState;
  degradeVfx?: boolean;
  isPaused: boolean;
  lowPowerMode: boolean;
  reducedMotion: boolean;
  sleepBreath: number;
  stars: CoCatStarParticle[];
  stateElapsedMs: number;
  updateProgress?: number;
}

export interface CoCatVfxParticleCounts {
  celebrateParticles: number;
  clickStars: number;
  coolingParticles: number;
  dataCubes: number;
  sparkParticles: number;
  steamPuffs: number;
  total: number;
}

export interface CoCatVfxSnapshot {
  activeEffects: CoCatVfxEffectId[];
  animationState: CoCatAnimationState;
  isAutoDegraded: boolean;
  isPaused: boolean;
  particleCounts: CoCatVfxParticleCounts;
  shouldRenderHighFrequencyVfx: boolean;
  sleepBreath: number;
  stars: CoCatStarParticle[];
  stateElapsedMs: number;
  updateProgress: number;
}
