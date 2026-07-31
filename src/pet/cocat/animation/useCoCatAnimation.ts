import { useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { CatState } from "../../../types/pet";
import type {
  CoCatAnimationContext,
  CoCatAnimationState,
  CoCatEyeState,
  CoCatPose,
  CoCatStarParticle,
} from "./animationTypes";
import {
  getCoCatOneShotDurationMs,
  getCoCatTransitionMs,
  resolveCoCatAnimationState,
} from "./animationStateMachine";
import { mixTransitionPose } from "./animationMixer";
import {
  applyCoCatActionOverlay,
  sampleCoCatPose,
} from "./animationRuntime";
import { CORE_CAT_ANIMATION_CONFIG } from "./animationConfig";
import {
  createCoCatActionControllerState,
  updateCoCatActionController,
} from "./animationActionController";
import { coCatVfxBus } from "../vfx/vfxBus";
import { getCoCatStateVfxEvents } from "../vfx/vfxRuntime";
import type { CoCatVfxEvent } from "../vfx/vfxTypes";

export interface UseCoCatAnimationOptions {
  catState: CatState;
  debugStateOverride?: CoCatAnimationState | null;
  getOneShotDurationMs?: (state: CoCatAnimationState) => number | null;
  interactionStateOverride?: CoCatAnimationState | null;
  interactionStateRequestId?: number;
  pointerInside: boolean;
  pointerOffset: { x: number; y: number };
  isClicking: boolean;
  isDragging: boolean;
  staticMode: boolean;
  lowPowerMode: boolean;
  updateProgress?: number;
}

export interface CoCatAnimationFrame {
  animationState: CoCatAnimationState;
  eyeState: CoCatEyeState;
  isGoggleShimmering: boolean;
  pose: CoCatPose;
  previousAnimationState: CoCatAnimationState;
  reducedMotion: boolean;
  sleepBreath: number;
  stars: CoCatStarParticle[];
  stateElapsedMs: number;
  transitionDurationMs: number;
}

const initialFrame: CoCatAnimationFrame = {
  animationState: "idle",
  eyeState: "normal",
  isGoggleShimmering: false,
  pose: {},
  previousAnimationState: "idle",
  reducedMotion: false,
  sleepBreath: 0,
  stars: [],
  stateElapsedMs: 0,
  transitionDurationMs: 0,
};

interface StandbyLoopState {
  state: StandbyAnimationState;
  until: number;
}

type StandbyAnimationState = "idle" | "scaredByMouse" | "eatingFish";

const standbyAnimationStates: StandbyAnimationState[] = [
  "idle",
  "scaredByMouse",
  "eatingFish",
];

export function useCoCatAnimation(options: UseCoCatAnimationOptions) {
  const [frame, setFrame] = useState<CoCatAnimationFrame>(initialFrame);
  const optionsRef = useRef(options);
  const currentPoseRef = useRef<CoCatPose>({});
  const transitionFromPoseRef = useRef<CoCatPose>({});
  const transitionStartedAtRef = useRef(0);
  const transitionDurationRef = useRef(0);
  const stateStartedAtRef = useRef(performance.now());
  const stateRef = useRef<CoCatAnimationState>("idle");
  const blinkUntilRef = useRef(0);
  const earTwitchRef = useRef<{
    side: "left" | "right";
    startedAt: number;
    until: number;
  } | null>(null);
  const shimmerUntilRef = useRef(0);
  const starsRef = useRef<CoCatStarParticle[]>([]);
  const actionControllerRef = useRef(createCoCatActionControllerState());
  const reducedMotionRef = useRef(false);
  const completedOneShotStateRef = useRef<CoCatAnimationState | null>(null);
  const pendingStateRef = useRef<CoCatAnimationState | null>(null);
  const previousCatStateRef = useRef<CatState>(options.catState);
  const previousDebugStateRef = useRef<CoCatAnimationState | null>(
    options.debugStateOverride ?? null,
  );
  const previousInteractionStateRef = useRef<CoCatAnimationState | null>(
    options.interactionStateOverride ?? null,
  );
  const previousInteractionRequestIdRef = useRef(
    options.interactionStateRequestId ?? 0,
  );
  const previousAnimationStateRef = useRef<CoCatAnimationState>("idle");
  const standbyLoopRef = useRef<StandbyLoopState>({
    state: "idle",
    until: 0,
  });
  const wasPointerInsideRef = useRef(false);
  const loopCueBucketsRef = useRef<Record<string, number>>({});

  optionsRef.current = options;

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined;
    }

    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    reducedMotionRef.current = mediaQuery.matches;

    const handleReducedMotionChange = (event: MediaQueryListEvent) => {
      reducedMotionRef.current = event.matches;
    };

    mediaQuery.addEventListener("change", handleReducedMotionChange);
    return () => mediaQuery.removeEventListener("change", handleReducedMotionChange);
  }, []);

  useEffect(() => {
    let disposed = false;
    let blinkTimer: number | undefined;
    let earTimer: number | undefined;

    function scheduleBlink() {
      blinkTimer = window.setTimeout(
        () => {
          if (disposed) {
            return;
          }
          blinkUntilRef.current =
            performance.now() + CORE_CAT_ANIMATION_CONFIG.idle.blinkDurationMs;
          scheduleBlink();
        },
        randomInRange(
          CORE_CAT_ANIMATION_CONFIG.idle.blinkMinMs,
          CORE_CAT_ANIMATION_CONFIG.idle.blinkMaxMs,
        ),
      );
    }

    function scheduleEarTwitch() {
      earTimer = window.setTimeout(
        () => {
          if (disposed) {
            return;
          }
          const now = performance.now();
          earTwitchRef.current = {
            side: Math.random() > 0.5 ? "left" : "right",
            startedAt: now,
            until: now + CORE_CAT_ANIMATION_CONFIG.idle.earTwitchDurationMs,
          };
          scheduleEarTwitch();
        },
        randomInRange(
          CORE_CAT_ANIMATION_CONFIG.idle.earTwitchMinMs,
          CORE_CAT_ANIMATION_CONFIG.idle.earTwitchMaxMs,
        ),
      );
    }

    scheduleBlink();
    scheduleEarTwitch();

    return () => {
      disposed = true;
      if (blinkTimer != null) {
        window.clearTimeout(blinkTimer);
      }
      if (earTimer != null) {
        window.clearTimeout(earTimer);
      }
    };
  }, []);

  useEffect(() => {
    let tickTimer: number | undefined;
    let disposed = false;

    function tick() {
      if (disposed) {
        return;
      }

      const now = performance.now();
      const nextFrame = sampleFrame(now);
      setFrame(nextFrame);
      tickTimer = window.setTimeout(
        tick,
        getAnimationTickIntervalMs(optionsRef.current, reducedMotionRef.current),
      );
    }

    tick();

    return () => {
      disposed = true;
      if (tickTimer != null) {
        window.clearTimeout(tickTimer);
      }
    };
  }, []);

  function sampleFrame(now: number): CoCatAnimationFrame {
    const currentOptions = optionsRef.current;
    const currentInteraction = currentOptions.interactionStateOverride ?? null;
    const currentInteractionRequestId =
      currentOptions.interactionStateRequestId ?? 0;
    const prevInteraction = previousInteractionStateRef.current;
    const prevInteractionRequestId = previousInteractionRequestIdRef.current;
    const catStateChanged =
      previousCatStateRef.current !== currentOptions.catState;
    const debugStateChanged =
      previousDebugStateRef.current !== (currentOptions.debugStateOverride ?? null);
    const interactionStarted =
      currentInteraction !== null &&
      (prevInteraction !== currentInteraction ||
        prevInteractionRequestId !== currentInteractionRequestId);
    const interactionEnded =
      prevInteraction !== null && currentInteraction === null;

    if (catStateChanged || debugStateChanged || interactionStarted) {
      // catState/debug 变化，或新的 interaction 动画开始 → 完全重置
      completedOneShotStateRef.current = null;
      pendingStateRef.current = null;
    } else if (interactionEnded) {
      // interaction 计时器正常结束（从有值变为 null）→ 只更新追踪引用，
      // 不清 completedOneShotState / pendingState，让排队中的下一状态继续等待
    }

    if (
      catStateChanged ||
      debugStateChanged ||
      prevInteraction !== currentInteraction ||
      prevInteractionRequestId !== currentInteractionRequestId
    ) {
      previousCatStateRef.current = currentOptions.catState;
      previousDebugStateRef.current = currentOptions.debugStateOverride ?? null;
      previousInteractionStateRef.current = currentInteraction;
      previousInteractionRequestIdRef.current = currentInteractionRequestId;
    }

    const actionUpdate = updateCoCatActionController(
      actionControllerRef.current,
      currentOptions.isClicking,
      now,
    );
    actionControllerRef.current = actionUpdate.state;

    const activeState = stateRef.current;
    const activeStateElapsedMs = now - stateStartedAtRef.current;
    const activeOneShotDurationMs = getResolvedOneShotDurationMs(
      currentOptions,
      activeState,
    );

    if (
      activeOneShotDurationMs != null &&
      activeStateElapsedMs >= activeOneShotDurationMs &&
      completedOneShotStateRef.current !== activeState &&
      currentInteraction !== activeState
    ) {
      completedOneShotStateRef.current = activeState;
    }

    const resolvedState = resolveStandbyLoopState(
      resolveCoCatAnimationState({
        catState: currentOptions.catState,
        completedOneShotState: completedOneShotStateRef.current,
        debugStateOverride: currentOptions.debugStateOverride ?? null,
        interactionStateOverride: currentOptions.interactionStateOverride ?? null,
        pointerInside: currentOptions.pointerInside,
        isClicking: actionUpdate.isClicking,
        isDragging: currentOptions.isDragging,
        staticMode: currentOptions.staticMode,
        lowPowerMode: currentOptions.lowPowerMode,
      }),
      currentOptions,
      actionUpdate.isClicking,
      now,
      standbyLoopRef,
    );
    const nextState = resolveQueuedCoCatState(
      activeState,
      resolvedState,
      activeStateElapsedMs,
      activeOneShotDurationMs,
      pendingStateRef,
    );

    if (currentOptions.pointerInside && !wasPointerInsideRef.current) {
      shimmerUntilRef.current = now + CORE_CAT_ANIMATION_CONFIG.hover.shimmerMs;
      coCatVfxBus.emit({ type: "goggleShimmer" });
    }
    wasPointerInsideRef.current = currentOptions.pointerInside;

    if (actionUpdate.didStartClick && currentOptions.catState !== "Hidden") {
      starsRef.current = createClickStars();
      coCatVfxBus.emit({
        count: starsRef.current.length,
        type: "clickStars",
      });
    }

    if (nextState !== stateRef.current) {
      previousAnimationStateRef.current = stateRef.current;
      transitionFromPoseRef.current = currentPoseRef.current;
      transitionStartedAtRef.current = now;
      transitionDurationRef.current = getCoCatTransitionMs(
        stateRef.current,
        nextState,
      );
      stateStartedAtRef.current = now;
      stateRef.current = nextState;
      getCoCatStateVfxEvents(nextState).forEach((event) =>
        coCatVfxBus.emit(event),
      );
    }

    const stateElapsedMs = now - stateStartedAtRef.current;
    emitLoopVfxCues(
      nextState,
      stateElapsedMs,
      loopCueBucketsRef.current,
    );

    const oneShotDurationMs = getResolvedOneShotDurationMs(
      currentOptions,
      nextState,
    );
    if (
      oneShotDurationMs != null &&
      stateElapsedMs >= oneShotDurationMs &&
      currentInteraction !== nextState
    ) {
      completedOneShotStateRef.current = nextState;
    }

    if (earTwitchRef.current && now > earTwitchRef.current.until) {
      earTwitchRef.current = null;
    }

    const context: CoCatAnimationContext = {
      now,
      state: nextState,
      stateElapsedMs,
      pointer: {
        isInside: currentOptions.pointerInside,
        x: currentOptions.pointerOffset.x,
        y: currentOptions.pointerOffset.y,
      },
      blinkActive: now < blinkUntilRef.current && nextState === "idle",
      earTwitch: earTwitchRef.current?.side ?? null,
      reducedMotion: reducedMotionRef.current,
      staticMode: currentOptions.staticMode,
      lowPowerMode: currentOptions.lowPowerMode,
      isDragging: currentOptions.isDragging,
      isClicking: actionUpdate.isClicking,
      updateProgress: currentOptions.updateProgress ?? 0,
    };

    const targetPose = sampleCoCatPose(nextState, context);

    const transitionElapsed = now - transitionStartedAtRef.current;
    const transitionPose = mixTransitionPose(
      transitionFromPoseRef.current,
      targetPose,
      transitionElapsed,
      transitionDurationRef.current,
    );
    const pose = applyCoCatActionOverlay(nextState, transitionPose, context);
    currentPoseRef.current = pose;

    return {
      animationState: nextState,
      eyeState: resolveEyeState(nextState, now < blinkUntilRef.current),
      isGoggleShimmering: now < shimmerUntilRef.current,
      pose,
      previousAnimationState: previousAnimationStateRef.current,
      reducedMotion: reducedMotionRef.current,
      sleepBreath: getSleepBreath(now),
      stars:
        nextState === "sleep" || nextState === "lowPowerStatic"
          ? []
          : starsRef.current,
      stateElapsedMs,
      transitionDurationMs: transitionDurationRef.current,
    };
  }

  return frame;
}

function resolveQueuedCoCatState(
  activeState: CoCatAnimationState,
  resolvedState: CoCatAnimationState,
  activeStateElapsedMs: number,
  activeOneShotDurationMs: number | null,
  pendingStateRef: MutableRefObject<CoCatAnimationState | null>,
) {
  if (
    activeOneShotDurationMs != null &&
    activeStateElapsedMs < activeOneShotDurationMs
  ) {
    if (resolvedState !== activeState) {
      pendingStateRef.current = resolvedState;
    }

    return activeState;
  }

  const pendingState = pendingStateRef.current;
  if (pendingState && pendingState !== activeState) {
    pendingStateRef.current = null;
    return pendingState;
  }

  pendingStateRef.current = null;
  return resolvedState;
}

function getResolvedOneShotDurationMs(
  options: UseCoCatAnimationOptions,
  state: CoCatAnimationState,
) {
  return (
    options.getOneShotDurationMs?.(state) ?? getCoCatOneShotDurationMs(state)
  );
}

function resolveStandbyLoopState(
  resolvedState: CoCatAnimationState,
  options: UseCoCatAnimationOptions,
  isClicking: boolean,
  now: number,
  standbyLoopRef: MutableRefObject<StandbyLoopState>,
): CoCatAnimationState {
  if (
    resolvedState !== "idle" ||
    options.catState !== "Idle" ||
    options.debugStateOverride ||
    options.interactionStateOverride ||
    options.pointerInside ||
    isClicking ||
    options.isDragging ||
    options.staticMode ||
    options.lowPowerMode
  ) {
    standbyLoopRef.current = { state: "idle", until: 0 };
    return resolvedState;
  }

  if (now >= standbyLoopRef.current.until) {
    standbyLoopRef.current = {
      state: pickRandomStandbyAnimationState(),
      until: now + randomInRange(5000, 10000),
    };
  }

  return standbyLoopRef.current.state;
}

function pickRandomStandbyAnimationState(): StandbyAnimationState {
  return standbyAnimationStates[
    Math.floor(Math.random() * standbyAnimationStates.length)
  ];
}

function resolveEyeState(
  state: CoCatAnimationState,
  blinkActive: boolean,
): CoCatEyeState {
  if (state === "sleep" || state === "lowPowerStatic") {
    return "sleepy";
  }

  if (state === "click") {
    return "focused";
  }

  if (state === "memoryCrowded") {
    return "dizzy";
  }

  if (state === "errorGlitch") {
    return "dizzy";
  }

  if (
    state === "temperatureCheck" ||
    state === "celebrate" ||
    state === "workshopUpgrade" ||
    state === "moduleUpgrade" ||
    state === "achievementPop"
  ) {
    return "glowing";
  }

  if (
    state === "repairing" ||
    state === "dataSorting" ||
    state === "panelOpen" ||
    state === "panelClose" ||
    state === "updateInstalling"
  ) {
    return "focused";
  }

  return blinkActive ? "blink" : "normal";
}

function getSleepBreath(now: number) {
  const durationMs = CORE_CAT_ANIMATION_CONFIG.sleep.breathMs;
  return 0.5 - Math.cos(((now % durationMs) / durationMs) * Math.PI * 2) / 2;
}

function createClickStars(): CoCatStarParticle[] {
  const config = CORE_CAT_ANIMATION_CONFIG.click;
  const count =
    config.starMinCount +
    Math.floor(Math.random() * (config.starMaxCount - config.starMinCount + 1));

  return Array.from({ length: count }, (_, index) => ({
    id: `${Date.now()}-${index}`,
    dx: 18 + Math.random() * 18,
    dy: -20 - Math.random() * 18,
    delayMs: index * config.starDelayStepMs,
  }));
}

function randomInRange(min: number, max: number) {
  return min + Math.random() * (max - min);
}

function getAnimationTickIntervalMs(
  options: UseCoCatAnimationOptions,
  reducedMotion: boolean,
) {
  if (typeof document !== "undefined" && document.visibilityState === "hidden") {
    return 1000;
  }

  if (options.staticMode || reducedMotion) {
    return 500;
  }

  if (options.lowPowerMode) {
    return 250;
  }

  if (
    options.isDragging ||
    options.isClicking ||
    options.pointerInside ||
    options.interactionStateOverride
  ) {
    return 50;
  }

  return 83;
}

function emitLoopVfxCues(
  state: CoCatAnimationState,
  stateElapsedMs: number,
  buckets: Record<string, number>,
) {
  if (state === "repairing") {
    emitEvery(state, stateElapsedMs, 160, { count: 4, type: "spark" }, buckets);
  }

  if (state === "memoryCrowded") {
    emitEvery(
      state,
      stateElapsedMs,
      800,
      { count: 3, type: "steamBurst" },
      buckets,
    );
  }

  if (state === "dataSorting") {
    emitEvery(
      state,
      stateElapsedMs,
      650,
      { count: 6, type: "dataCubeSpawn" },
      buckets,
    );
  }

  if (state === "updateInstalling") {
    emitEvery(
      state,
      stateElapsedMs,
      900,
      { type: "updateProgress" },
      buckets,
    );
  }
}

function emitEvery(
  state: CoCatAnimationState,
  stateElapsedMs: number,
  intervalMs: number,
  event: CoCatVfxEvent,
  buckets: Record<string, number>,
) {
  const key = `${state}:${event.type}`;
  const bucket = Math.floor(stateElapsedMs / intervalMs);
  if (buckets[key] === bucket) {
    return;
  }

  buckets[key] = bucket;
  coCatVfxBus.emit(event);
}
