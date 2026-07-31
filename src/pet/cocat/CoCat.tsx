import { useCallback, useEffect, useMemo, useRef } from "react";
import type { CatState } from "../../types/pet";
import { CoCatVfxLayer } from "./vfx/CoCatVfxLayer";
import { useCoCatAnimation } from "./animation/useCoCatAnimation";
import type {
  CoCatAnimationState,
} from "./animation/animationTypes";
import { createCoCatVfxSnapshot } from "./vfx/vfxRuntime";
import { useCoCatPerformance } from "./performance/useCoCatPerformance";
import type { CoCatPerformanceReport } from "./performance/coCatPerformanceTypes";
import {
  getSpriteSheetForState,
  getSpriteSheetOneShotDurationMs,
  isSpriteSheetOneShot,
  preloadSpriteSheetForState,
} from "./animation/spriteSheetAssets";
import { getCoCatOneShotDurationMs } from "./animation/animationStateMachine";
import {
  getSpriteFrameStyle,
  resolveSpriteFrameIndex,
  resolveSpriteFrameIndexLowPower,
} from "./animation/spriteSheetPlayer";
import "./cocat.css";

const coCatInternalSize = 160;
const coCatRenderScale = 0.74;
const coCatRenderSize = Math.round(coCatInternalSize * coCatRenderScale);

export interface CoCatDebugInfo {
  activeVfxCount: number;
  animationState: CoCatAnimationState;
  isLowPower: boolean;
  isVfxAutoDegraded: boolean;
  isVfxPaused: boolean;
  previousAnimationState: CoCatAnimationState;
  reducedMotion: boolean;
  stateElapsedMs: number;
  transitionDurationMs: number;
  vfxParticleCount: number;
}

export interface CoCatProps {
  catState: CatState;
  debugPauseVfx?: boolean;
  debugShowBounds?: boolean;
  debugShowVfxAnchors?: boolean;
  debugStateOverride?: CoCatAnimationState | null;
  debugUpdateProgress?: number;
  degradeVfx?: boolean;
  interactionStateOverride?: CoCatAnimationState | null;
  interactionStateRequestId?: number;
  isClicking: boolean;
  isDragging: boolean;
  lowPowerMode: boolean;
  onDebugInfo?: (info: CoCatDebugInfo) => void;
  onPerformanceReport?: (report: CoCatPerformanceReport) => void;
  pointerInside: boolean;
  pointerOffset: { x: number; y: number };
  staticMode: boolean;
}

export function CoCat({
  catState,
  debugPauseVfx = false,
  debugShowBounds = false,
  debugShowVfxAnchors = false,
  debugStateOverride = null,
  debugUpdateProgress = 0,
  degradeVfx = false,
  interactionStateOverride = null,
  interactionStateRequestId = 0,
  isClicking,
  isDragging,
  lowPowerMode,
  onDebugInfo,
  onPerformanceReport,
  pointerInside,
  pointerOffset,
  staticMode,
}: CoCatProps) {
  const getOneShotDurationMs = useCallback(
    (state: CoCatAnimationState) => {
      if (state === "click") {
        return getCoCatOneShotDurationMs(state);
      }

      return (
        getSpriteSheetOneShotDurationMs(state) ??
        getCoCatOneShotDurationMs(state)
      );
    },
    [],
  );

  const {
    animationState,
    eyeState,
    pose,
    previousAnimationState,
    reducedMotion,
    sleepBreath,
    stars,
    stateElapsedMs,
    transitionDurationMs,
  } = useCoCatAnimation({
    catState,
    debugStateOverride,
    getOneShotDurationMs,
    interactionStateOverride,
    interactionStateRequestId,
    isClicking,
    isDragging,
    lowPowerMode,
    pointerInside,
    pointerOffset,
    staticMode,
    updateProgress: debugUpdateProgress,
  });

  const vfxSnapshot = createCoCatVfxSnapshot({
    animationState,
    degradeVfx: degradeVfx || lowPowerMode || staticMode,
    isPaused: debugPauseVfx || staticMode,
    lowPowerMode,
    reducedMotion,
    sleepBreath,
    stars,
    stateElapsedMs,
    updateProgress: debugUpdateProgress,
  });

  const performanceReport = useCoCatPerformance({
    activeVfxCount: vfxSnapshot.activeEffects.length,
    animationState,
    isLowPower: lowPowerMode,
    isVfxAutoDegraded: vfxSnapshot.isAutoDegraded,
    isVfxPaused: debugPauseVfx,
    previousAnimationState,
    reducedMotion,
    transitionDurationMs,
    vfxParticleCount: vfxSnapshot.particleCounts.total,
  });

  useEffect(() => {
    onDebugInfo?.({
      activeVfxCount: vfxSnapshot.activeEffects.length,
      animationState,
      isLowPower: lowPowerMode,
      isVfxAutoDegraded: vfxSnapshot.isAutoDegraded,
      isVfxPaused: debugPauseVfx,
      previousAnimationState,
      reducedMotion,
      stateElapsedMs,
      transitionDurationMs,
      vfxParticleCount: vfxSnapshot.particleCounts.total,
    });
  }, [
    animationState,
    debugPauseVfx,
    lowPowerMode,
    onDebugInfo,
    previousAnimationState,
    reducedMotion,
    stateElapsedMs,
    transitionDurationMs,
    vfxSnapshot.activeEffects.length,
    vfxSnapshot.isAutoDegraded,
    vfxSnapshot.particleCounts.total,
  ]);

  useEffect(() => {
    onPerformanceReport?.(performanceReport);
  }, [onPerformanceReport, performanceReport]);

  // -----------------------------------------------------------------------
  // Sprite-sheet frame resolution
  // -----------------------------------------------------------------------

  // Detect rapid clicks for Click_Dizzy variant.
  // Track recent click timestamps and switch to dizzy when ≥3 clicks within 1.5s.
  const clickTimestampsRef = useRef<number[]>([]);
  const lastIsClickingRef = useRef(false);
  const rapidClickThreshold = 3;
  const rapidClickWindowMs = 1500;

  useEffect(() => {
    if (isClicking && !lastIsClickingRef.current) {
      const now = performance.now();
      const timestamps = clickTimestampsRef.current;
      timestamps.push(now);
      while (timestamps.length > 0 && now - timestamps[0] > rapidClickWindowMs) {
        timestamps.shift();
      }
    }
    lastIsClickingRef.current = isClicking;
  }, [isClicking]);

  const useClickDizzy =
    animationState === "click" &&
    clickTimestampsRef.current.length >= rapidClickThreshold;

  useEffect(() => {
    preloadSpriteSheetForState(animationState, useClickDizzy);
  }, [animationState, useClickDizzy]);

  const spriteAsset = useMemo(
    () => getSpriteSheetForState(animationState, useClickDizzy),
    [animationState, useClickDizzy],
  );

  const loop = !isSpriteSheetOneShot(animationState);

  const spriteFrameStyle = useMemo(() => {
    if (!spriteAsset) {
      return null;
    }

    const frameIndex = lowPowerMode
      ? resolveSpriteFrameIndexLowPower(spriteAsset.meta, stateElapsedMs, loop)
      : resolveSpriteFrameIndex(spriteAsset.meta, stateElapsedMs, loop);

    return getSpriteFrameStyle(spriteAsset.meta, frameIndex, spriteAsset.sheetUrl);
  }, [spriteAsset, stateElapsedMs, loop, lowPowerMode]);

  // Derive pose-driven transforms from the animation runtime to add subtle
  // movement on top of the sprite-sheet frames. We read root and body_base
  // pose channels and composite them into a single CSS transform.
  const rootPose = pose.root ?? {};
  const bodyPose = pose.body_base ?? {};

  const poseTranslateX = (rootPose.x ?? 0) + (bodyPose.x ?? 0);
  const poseTranslateY = (rootPose.y ?? 0) + (bodyPose.y ?? 0);
  const poseScaleX = (rootPose.scaleX ?? 1) * (bodyPose.scaleX ?? 1);
  const poseScaleY = (rootPose.scaleY ?? 1) * (bodyPose.scaleY ?? 1);
  const poseRotate = (rootPose.rotate ?? 0) + (bodyPose.rotate ?? 0);
  const poseOpacity = (rootPose.opacity ?? 1) * (bodyPose.opacity ?? 1);

  // Shadow pose
  const shadowPose = pose.shadow ?? {};
  const shadowOpacity = shadowPose.opacity ?? 0.42;
  const shadowScaleX = shadowPose.scaleX ?? 1;

  return (
    <div
      aria-label="CoCat Pixel Visor"
      className={`cocat-root cocat-state--${animationState} ${
        staticMode ? "is-static" : ""
      } ${lowPowerMode ? "is-low-power" : ""} ${
        debugShowVfxAnchors ? "show-vfx-anchors" : ""
      }`}
      data-animation-state={animationState}
      style={{ width: coCatRenderSize, height: coCatRenderSize }}
    >
      <div
        className="cocat-pixel-container"
        style={{
          bottom: 0,
          height: coCatInternalSize,
          left: "50%",
          position: "absolute",
          transform: `translateX(-50%) scale(${coCatRenderScale})`,
          transformOrigin: "bottom center",
          width: coCatInternalSize,
        }}
      >
        {/* Render ambient shadow at the bottom, driven by pose */}
        <span
          className="cwp-pet-shadow"
          style={{
            left: "14%",
            bottom: "4px",
            opacity: staticMode ? 0.42 : shadowOpacity,
            transform: staticMode ? undefined : `scaleX(${shadowScaleX})`,
            animation: "none",
          }}
        />

        {/* Render sprite-sheet frame with pose-driven transforms */}
        <div
          className={`cwp-pet-sprite ${
            animationState === "click" ? "is-nodding" : ""
          }`}
          role="img"
          aria-label="CoCat"
          style={{
            width: spriteFrameStyle?.width ?? 160,
            height: spriteFrameStyle?.height ?? 160,
            position: "relative",
            zIndex: 2,
            imageRendering: "pixelated",
            transformOrigin: "bottom center",
            opacity: staticMode ? 1 : poseOpacity,
            transform: staticMode
              ? undefined
              : `translate3d(${poseTranslateX}px, ${poseTranslateY}px, 0) scale(${poseScaleX}, ${poseScaleY}) rotate(${poseRotate}deg)`,
            transition: "opacity 0.12s ease",
            backgroundImage: spriteFrameStyle?.backgroundImage,
            backgroundPosition: spriteFrameStyle?.backgroundPosition,
            backgroundSize: spriteFrameStyle?.backgroundSize,
            backgroundRepeat: "no-repeat",
          }}
        />

        {/* Overlay the interactive VFX Layer (cooling wind, steam, sparks, badge, star burst) */}
        <CoCatVfxLayer
          showAnchors={debugShowVfxAnchors}
          snapshot={vfxSnapshot}
        />
      </div>
    </div>
  );
}
