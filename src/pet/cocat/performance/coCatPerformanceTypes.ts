import type { CoCatAnimationState } from "../animation/animationTypes";

export interface CoCatStateTransitionRecord {
  atMs: number;
  durationMs: number;
  from: CoCatAnimationState;
  to: CoCatAnimationState;
}

export interface CoCatPerformanceFrameInput {
  activeVfxCount: number;
  animationState: CoCatAnimationState;
  isLowPower: boolean;
  isVfxAutoDegraded: boolean;
  isVfxPaused: boolean;
  previousAnimationState: CoCatAnimationState;
  reducedMotion: boolean;
  transitionDurationMs: number;
  vfxParticleCount: number;
}

export interface CoCatPerformanceReport
  extends CoCatPerformanceFrameInput {
  averageFrameMs: number;
  fps: number;
  frameCount: number;
  sampledAtMs: number;
  stateTransitions: CoCatStateTransitionRecord[];
}
