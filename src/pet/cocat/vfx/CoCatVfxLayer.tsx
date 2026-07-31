import { CelebrateBurst } from "./effects/CelebrateBurst";
import { AchievementBadge } from "./effects/AchievementBadge";
import { ClickStars } from "./effects/ClickStars";
import { CoolingParticles } from "./effects/CoolingParticles";
import { CoolingText } from "./effects/CoolingText";
import { CoolingWind } from "./effects/CoolingWind";
import { DataCubes } from "./effects/DataCubes";
import { ErrorGlitch } from "./effects/ErrorGlitch";
import { HologramPanel } from "./effects/HologramPanel";
import { MemoryRamBox } from "./effects/MemoryRamBox";
import { MemorySteam } from "./effects/MemorySteam";
import { RepairSparks } from "./effects/RepairSparks";
import { SleepBubble } from "./effects/SleepBubble";
import { UpdateProgress } from "./effects/UpdateProgress";
import type { CoCatVfxEffectId, CoCatVfxSnapshot } from "./vfxTypes";

export interface CoCatVfxLayerProps {
  snapshot: CoCatVfxSnapshot;
  showAnchors: boolean;
}

export function CoCatVfxLayer({
  showAnchors,
  snapshot,
}: CoCatVfxLayerProps) {
  const hasEffect = (effect: CoCatVfxEffectId) =>
    snapshot.activeEffects.includes(effect);

  return (
    <div
      aria-hidden="true"
      className={`cocat-vfx-layer ${snapshot.isPaused ? "is-paused" : ""}`}
      data-vfx-count={snapshot.activeEffects.length}
    >
      {showAnchors ? <span className="cocat-vfx-debug-anchor" /> : null}
      {hasEffect("sleepBubble") ? (
        <SleepBubble sleepBreath={snapshot.sleepBreath} />
      ) : null}
      {hasEffect("clickStars") ? <ClickStars stars={snapshot.stars} /> : null}
      {hasEffect("coolingWind") ? <CoolingWind /> : null}
      {hasEffect("coolingParticles") ? (
        <CoolingParticles count={snapshot.particleCounts.coolingParticles} />
      ) : null}
      {hasEffect("coolingText") ? <CoolingText /> : null}
      {hasEffect("memoryRamBox") ? <MemoryRamBox /> : null}
      {hasEffect("memorySteam") ? (
        <MemorySteam count={snapshot.particleCounts.steamPuffs} />
      ) : null}
      {hasEffect("repairSparks") ? (
        <RepairSparks count={snapshot.particleCounts.sparkParticles} />
      ) : null}
      {hasEffect("hologramPanel") ? <HologramPanel /> : null}
      {hasEffect("dataCubes") ? (
        <DataCubes count={snapshot.particleCounts.dataCubes} />
      ) : null}
      {hasEffect("celebrateBurst") ? (
        <CelebrateBurst count={snapshot.particleCounts.celebrateParticles} />
      ) : null}
      {hasEffect("errorGlitch") ? <ErrorGlitch /> : null}
      {hasEffect("updateProgress") ? (
        <UpdateProgress progress={snapshot.updateProgress} />
      ) : null}
      {hasEffect("achievementBadge") ? <AchievementBadge /> : null}
    </div>
  );
}
