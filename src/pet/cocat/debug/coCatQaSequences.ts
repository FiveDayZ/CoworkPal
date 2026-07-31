import type { CoCatAnimationState } from "../animation/animationTypes";
import { getCoCatOneShotDurationMs } from "../animation/animationStateMachine";
import { CORE_CAT_ANIMATION_CONFIG } from "../animation/animationConfig";

export type CoCatQaSequenceId = "all" | "hardware" | "interaction";

export interface CoCatQaSequence {
  id: CoCatQaSequenceId;
  label: string;
  states: CoCatAnimationState[];
}

export const CORE_CAT_QA_SEQUENCES: Record<
  CoCatQaSequenceId,
  CoCatQaSequence
> = {
  all: {
    id: "all",
    label: "All States",
    states: [
      "idle",
      "scaredByMouse",
      "eatingFish",
      "dataSorting",
      "memoryCrowded",
      "temperatureCheck",
      "repairing",
      "celebrate",
      "workshopUpgrade",
      "moduleUpgrade",
      "lowPowerStatic",
      "bootWake",
      "hover",
      "click",
      "panelOpen",
      "panelClose",
      "pettingHearts",
      "dragging",
      "dropLanding",
      "errorGlitch",
      "updateInstalling",
      "achievementPop",
      "idle",
    ],
  },
  hardware: {
    id: "hardware",
    label: "Hardware States",
    states: [
      "idle",
      "scaredByMouse",
      "eatingFish",
      "memoryCrowded",
      "temperatureCheck",
      "repairing",
      "celebrate",
      "idle",
    ],
  },
  interaction: {
    id: "interaction",
    label: "Interaction States",
    states: [
      "bootWake",
      "hover",
      "click",
      "panelOpen",
      "panelClose",
      "pettingHearts",
      "dragging",
      "dropLanding",
      "lowPowerStatic",
      "idle",
    ],
  },
};

export function getCoCatQaSequence(id: CoCatQaSequenceId) {
  return CORE_CAT_QA_SEQUENCES[id];
}

export function getCoCatQaStepHoldMs(state: CoCatAnimationState) {
  const oneShotMs = getCoCatOneShotDurationMs(state);
  if (oneShotMs != null) {
    return oneShotMs + CORE_CAT_ANIMATION_CONFIG.transition.defaultMs + 80;
  }

  if (state === "click") {
    return CORE_CAT_ANIMATION_CONFIG.click.totalMs + 120;
  }

  return 620;
}

export function shouldCoCatQaStepAutoFallback(
  state: CoCatAnimationState,
) {
  return getCoCatOneShotDurationMs(state) != null;
}
