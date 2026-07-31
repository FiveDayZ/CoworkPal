import type { CoCatAnimationState } from "../animation/animationTypes";
import { CORE_CAT_ANIMATION_CONFIG } from "../animation/animationConfig";
import type {
  CoCatVfxEffectId,
  CoCatVfxEvent,
  CoCatVfxParticleCounts,
  CoCatVfxRuntimeInput,
  CoCatVfxSnapshot,
} from "./vfxTypes";

export const COCAT_VFX_LIMITS = {
  maxActiveParticles: 80,
  maxCoolingParticles: 24,
  maxDataCubes: 16,
  maxSteamPuffs: 8,
  maxSparkParticles: 24,
  maxCelebrateParticles: 48,
} as const;

const COCAT_VFX_BASE_PARTICLE_COUNTS = {
  celebrateParticles: 16,
  coolingParticles: 12,
  dataCubes: 8,
  sparkParticles: 4,
  steamPuffs: 3,
} as const;

const highFrequencyStates = new Set<CoCatAnimationState>([
  "temperatureCheck",
  "memoryCrowded",
  "repairing",
  "dataSorting",
  "celebrate",
  "updateInstalling",
  "achievementPop",
  "errorGlitch",
]);

export function createCoCatVfxSnapshot(
  input: CoCatVfxRuntimeInput,
): CoCatVfxSnapshot {
  const shouldRenderHighFrequencyVfx =
    !input.isPaused &&
    !input.lowPowerMode &&
    input.animationState !== "sleep" &&
    !input.reducedMotion;
  const activeEffects: CoCatVfxEffectId[] = [];
  const requestedDegrade = Boolean(input.degradeVfx);
  const qualityMultiplier = input.lowPowerMode ? 0.35 : requestedDegrade ? 0.5 : 1;

  if (input.animationState === "sleep") {
    activeEffects.push("sleepBubble");
  }

  if (input.stars.length > 0 && !input.isPaused && input.animationState !== "sleep") {
    activeEffects.push("clickStars");
  }

  if (!input.isPaused && input.animationState === "errorGlitch") {
    activeEffects.push("errorGlitch");
  }

  if (!input.isPaused && input.animationState === "updateInstalling") {
    activeEffects.push("updateProgress");
  }

  if (!input.isPaused && input.animationState === "achievementPop") {
    activeEffects.push("achievementBadge");
  }

  if (shouldRenderHighFrequencyVfx) {
    if (input.animationState === "temperatureCheck") {
      activeEffects.push("coolingWind", "coolingParticles", "coolingText");
    }

    if (input.animationState === "memoryCrowded") {
      activeEffects.push("memoryRamBox", "memorySteam");
    }

    if (input.animationState === "repairing") {
      activeEffects.push("repairSparks", "hologramPanel");
    }

    if (input.animationState === "dataSorting") {
      activeEffects.push("dataCubes");
    }

    if (
      input.animationState === "celebrate" &&
      input.stateElapsedMs >= CORE_CAT_ANIMATION_CONFIG.celebrate.burstStartMs
    ) {
      activeEffects.push("celebrateBurst");
    }
  }

  if (!input.isPaused && input.lowPowerMode && !input.reducedMotion) {
    if (input.animationState === "repairing") {
      activeEffects.push("repairSparks");
    }

    if (
      input.animationState === "celebrate" &&
      input.stateElapsedMs >= CORE_CAT_ANIMATION_CONFIG.celebrate.burstStartMs
    ) {
      activeEffects.push("celebrateBurst");
    }
  }

  const particleCounts = createCoCatVfxParticleCounts(
    activeEffects,
    input.stars.length,
    qualityMultiplier,
  );
  const maxStars = Math.max(
    0,
    COCAT_VFX_LIMITS.maxActiveParticles -
      (particleCounts.total - particleCounts.clickStars),
  );
  const stars = input.stars.slice(0, Math.min(input.stars.length, maxStars));
  particleCounts.clickStars = stars.length;
  particleCounts.total =
    particleCounts.celebrateParticles +
    particleCounts.clickStars +
    particleCounts.coolingParticles +
    particleCounts.dataCubes +
    particleCounts.sparkParticles +
    particleCounts.steamPuffs;

  return {
    activeEffects,
    animationState: input.animationState,
    isAutoDegraded:
      requestedDegrade ||
      input.stars.length > stars.length ||
      particleCounts.total >= COCAT_VFX_LIMITS.maxActiveParticles,
    isPaused: input.isPaused,
    particleCounts,
    shouldRenderHighFrequencyVfx,
    sleepBreath: input.sleepBreath,
    stars,
    stateElapsedMs: input.stateElapsedMs,
    updateProgress: input.updateProgress ?? 0,
  };
}

export function createCoCatVfxParticleCounts(
  activeEffects: CoCatVfxEffectId[],
  requestedStars: number,
  qualityMultiplier = 1,
): CoCatVfxParticleCounts {
  const hasEffect = (effect: CoCatVfxEffectId) =>
    activeEffects.includes(effect);
  const limit = (base: number, max: number) =>
    Math.min(max, Math.max(0, Math.ceil(base * qualityMultiplier)));

  const counts = {
    celebrateParticles: hasEffect("celebrateBurst")
      ? limit(
          COCAT_VFX_BASE_PARTICLE_COUNTS.celebrateParticles,
          COCAT_VFX_LIMITS.maxCelebrateParticles,
        )
      : 0,
    clickStars: Math.min(requestedStars, COCAT_VFX_LIMITS.maxActiveParticles),
    coolingParticles: hasEffect("coolingParticles")
      ? limit(
          COCAT_VFX_BASE_PARTICLE_COUNTS.coolingParticles,
          COCAT_VFX_LIMITS.maxCoolingParticles,
        )
      : 0,
    dataCubes: hasEffect("dataCubes")
      ? limit(
          COCAT_VFX_BASE_PARTICLE_COUNTS.dataCubes,
          COCAT_VFX_LIMITS.maxDataCubes,
        )
      : 0,
    sparkParticles: hasEffect("repairSparks")
      ? limit(
          COCAT_VFX_BASE_PARTICLE_COUNTS.sparkParticles,
          COCAT_VFX_LIMITS.maxSparkParticles,
        )
      : 0,
    steamPuffs: hasEffect("memorySteam")
      ? limit(
          COCAT_VFX_BASE_PARTICLE_COUNTS.steamPuffs,
          COCAT_VFX_LIMITS.maxSteamPuffs,
        )
      : 0,
    total: 0,
  };

  counts.total =
    counts.celebrateParticles +
    counts.clickStars +
    counts.coolingParticles +
    counts.dataCubes +
    counts.sparkParticles +
    counts.steamPuffs;

  return counts;
}

export function getCoCatStateVfxEvents(
  state: CoCatAnimationState,
): CoCatVfxEvent[] {
  if (state === "temperatureCheck") {
    return [
      { type: "coolingWind", intensity: 1 },
      { type: "coldPixels", intensity: 1 },
    ];
  }

  if (state === "memoryCrowded") {
    return [{ type: "steamBurst", count: 3 }];
  }

  if (state === "repairing") {
    return [{ type: "spark", count: 4 }];
  }

  if (state === "dataSorting") {
    return [{ type: "dataCubeSpawn", count: 6 }];
  }

  if (state === "sleep") {
    return [{ type: "sleepBubblePulse" }];
  }

  if (state === "celebrate") {
    return [{ type: "goldenSteamRing" }];
  }

  if (state === "errorGlitch") {
    return [{ type: "errorGlitch" }];
  }

  if (state === "updateInstalling") {
    return [{ type: "updateProgress" }];
  }

  if (state === "achievementPop") {
    return [{ type: "achievementPop" }];
  }

  return [];
}

export function shouldPauseHighFrequencyVfx(
  state: CoCatAnimationState,
  lowPowerMode: boolean,
) {
  return lowPowerMode || state === "sleep" || !highFrequencyStates.has(state);
}
