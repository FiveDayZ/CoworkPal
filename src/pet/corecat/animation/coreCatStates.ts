import type { CatState } from "../../../types/pet";
import type { CoreCatAnimationState } from "./animationTypes";
import { CORE_CAT_ANIMATION_CONFIG } from "./animationConfig";

export const coreCatAnimationStates: CoreCatAnimationState[] = [
  "bootWake",
  "idle",
  "hover",
  "click",
  "dragging",
  "dropLanding",
  "panelOpen",
  "panelClose",
  "temperatureCheck",
  "memoryCrowded",
  "repairing",
  "dataSorting",
  "scaredByMouse",
  "eatingFish",
  "pettingHearts",
  "sleep",
  "celebrate",
  "workshopUpgrade",
  "moduleUpgrade",
  "updateInstalling",
  "achievementPop",
  "errorGlitch",
  "lowPowerStatic",
  "fatigued",
  "needsBreak",
  "freeMemory",
];

export const coreCatStatePriority: Record<CoreCatAnimationState, number> =
  CORE_CAT_ANIMATION_CONFIG.statePriority;

export function mapCatStateToCoreCatState(
  catState: CatState,
): CoreCatAnimationState {
  switch (catState) {
    case "TemperatureCheck":
      return "temperatureCheck";
    case "MemoryCrowded":
      return "memoryCrowded";
    case "DataSorting":
      return "dataSorting";
    case "Sleep":
    case "Hidden":
      return "sleep";
    case "Fatigued":
      return "fatigued";
    case "NeedsBreak":
      return "needsBreak";
    case "Celebrate":
      return "celebrate";
    case "RepairHeavy":
    case "RepairLight":
      return "repairing";
    case "DeepWork":
      // Heads-down focus: reuse the repairing animation (busy tinkering).
      return "repairing";
    case "Distracted":
      // Off-task: reuse idle; the bubble carries the nudge.
      return "idle";
    case "Interactive":
    case "Idle":
    default:
      return "idle";
  }
}
