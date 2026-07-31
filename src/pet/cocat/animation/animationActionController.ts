import { CORE_CAT_ANIMATION_CONFIG } from "./animationConfig";

export interface CoCatActionControllerState {
  clickUntilMs: number;
  previousClickInput: boolean;
}

export interface CoCatActionControllerResult {
  didStartClick: boolean;
  isClicking: boolean;
  state: CoCatActionControllerState;
}

export function createCoCatActionControllerState(): CoCatActionControllerState {
  return {
    clickUntilMs: 0,
    previousClickInput: false,
  };
}

export function updateCoCatActionController(
  state: CoCatActionControllerState,
  clickInput: boolean,
  now: number,
): CoCatActionControllerResult {
  const didStartClick = clickInput && !state.previousClickInput;
  const clickUntilMs = didStartClick
    ? now + CORE_CAT_ANIMATION_CONFIG.click.totalMs
    : state.clickUntilMs;

  return {
    didStartClick,
    isClicking: now < clickUntilMs,
    state: {
      clickUntilMs,
      previousClickInput: clickInput,
    },
  };
}
