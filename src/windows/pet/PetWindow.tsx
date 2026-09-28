import { useCallback, useEffect, useRef, useState } from "react";
import type { MouseEvent } from "react";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { playAudioFeedback } from "../../services/audioFeedback";
import {
  hidePetPanel,
  hidePetWindow,
  rewardCoCatInteraction,
  saveWindowPosition,
  showMainRoute,
  showPetPanel,
  trackAchievementEvent,
  triggerMemoryRelease,
} from "../../services/tauriCommands";
import {
  startDraggingCurrentWindow,
} from "../../services/windowApi";
import { useHardwareStore } from "../../stores/hardwareStore";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { CatState } from "../../types/pet";
import { ContextMenu, PetSpeechBubble } from "../../ui/components";
import { CoCat } from "../../pet/cocat/CoCat";
import type { CoCatDebugInfo } from "../../pet/cocat/CoCat";
import {
  CoCatDebugPanel,
  type CoCatDebugControls,
} from "../../pet/cocat/debug/CoCatDebugPanel";
import {
  CoCatQaPanel,
  type CoCatQaRunState,
} from "../../pet/cocat/debug/CoCatQaPanel";
import { CoCatAssetPanel } from "../../pet/cocat/debug/CoCatAssetPanel";
import {
  getCoCatQaSequence,
  getCoCatQaStepHoldMs,
  shouldCoCatQaStepAutoFallback,
  type CoCatQaSequenceId,
} from "../../pet/cocat/debug/coCatQaSequences";
import { coCatRuntimeAssetReport } from "../../pet/cocat/assets/coCatAssetModules";
import { CoCatPerformancePanel } from "../../pet/cocat/performance/CoCatPerformancePanel";
import type { CoCatPerformanceReport } from "../../pet/cocat/performance/coCatPerformanceTypes";
import { CORE_CAT_ANIMATION_CONFIG } from "../../pet/cocat/animation/animationConfig";
import type { CoCatAnimationState } from "../../pet/cocat/animation/animationTypes";
import { mapCatStateToCoCatState } from "../../pet/cocat/animation/coCatStates";
import { getCoCatOneShotDurationMs } from "../../pet/cocat/animation/animationStateMachine";
import { getSpriteSheetOneShotDurationMs } from "../../pet/cocat/animation/spriteSheetAssets";
import { resolveCoCatHoverHitArea } from "../../pet/cocat/animation/hoverHitArea";

type VisualCatState = CatState;
type InteractionRequest = {
  requestId: number;
  state: CoCatAnimationState;
};

const dragStartThresholdPx = 3;
const warningStates: CatState[] = [
  "RepairLight",
  "RepairHeavy",
  "TemperatureCheck",
  "MemoryCrowded",
];
const focusLowPowerDelayMs = 30_000;
const systemHighLoadCpuThreshold = 85;
const systemHighLoadMemoryThreshold = 90;
const compactPetWindowSize = {
  width: 196,
  height: 166,
};

const initialQaRun: CoCatQaRunState = {
  cleanupOk: null,
  isRunning: false,
  sequenceId: null,
  stuckState: null,
  steps: [],
};

export function PetWindow() {
  const snapshot = useHardwareStore((state) => state.snapshot);
  const catState = usePetStore((state) => state.catState);
  const catMessage = usePetStore((state) => state.catMessage);
  const rewardMessageUntil = usePetStore((state) => state.rewardMessageUntil);
  const markPanelOpen = usePetStore((state) => state.openPanel);
  const markPanelClosed = usePetStore((state) => state.closePanel);
  const settings = useSettingsStore((state) => state.settings);
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [isNodding, setIsNodding] = useState(false);
  const [isAlerting, setIsAlerting] = useState(false);
  const [bubbleVisible, setBubbleVisible] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [isWindowLowPower, setIsWindowLowPower] = useState(false);
  const [mouseOffset, setMouseOffset] = useState({ x: 0, y: 0 });
  const [interactionRequest, setInteractionRequest] =
    useState<InteractionRequest | null>(null);
  const [debugControls, setDebugControls] = useState<CoCatDebugControls>({
    forceLowPower: false,
    pauseVfx: false,
    showSkeletonBounds: false,
    showVfxAnchors: false,
    stateOverride: null,
    updateProgress: 0.35,
  });
  const [debugInfo, setDebugInfo] = useState<CoCatDebugInfo | null>(null);
  const [performanceReport, setPerformanceReport] =
    useState<CoCatPerformanceReport | null>(null);
  const [qaRun, setQaRun] = useState<CoCatQaRunState>(initialQaRun);
  const dragCandidateRef = useRef(false);
  const dragMovedRef = useRef(false);
  const dragStartRef = useRef({ x: 0, y: 0 });
  const dragStartAtRef = useRef(0);
  const dragDistanceRef = useRef(0);
  const petCanvasRef = useRef<HTMLDivElement | null>(null);
  const saveTimerRef = useRef<number | null>(null);
  const interactionTimerRef = useRef<number | null>(null);
  const noddingTimerRef = useRef<number | null>(null);
  const debugInfoTimerRef = useRef(0);
  const debugInfoStateRef = useRef<string | null>(null);
  const debugInfoRef = useRef<CoCatDebugInfo | null>(null);
  const qaCancelledRef = useRef(false);
  const windowLowPowerTimerRef = useRef<number | null>(null);
  const previousStateRef = useRef<VisualCatState>("Idle");
  // 用于检测 Tauri 原生拖拽释放：onMoved 停止触发后 120ms 视为拖拽结束
  const dropLandingTimerRef = useRef<number | null>(null);
  // 当一个 one-shot 动画正在播放时，新的 one-shot 请求排队等待
  const pendingInteractionStateRef = useRef<CoCatAnimationState | null>(null);
  const interactionRequestIdRef = useRef(0);
  const errorGlitchArmedRef = useRef(true);
  const clickBurstRef = useRef({ count: 0, startedAt: 0 });
  const reportedVisualAnimationRef = useRef<CoCatAnimationState | null>(null);
  // 自引用的动画结束处理函数（存在 ref 里以支持递归调用）
  const finishInteractionRef = useRef<(() => void) | null>(null);

  const visualState: VisualCatState =
    settings?.enableStaticCatMode && catState !== "Hidden"
      ? "Idle"
      : catState;
  const scale = settings?.catSize ?? 1;
  const opacity = settings?.catOpacity ?? 0.95;
  const bubbleEnabled = settings?.enablePetBubble ?? true;
  const staticModeEnabled = settings?.enableStaticCatMode ?? false;
  const manualLowPowerModeEnabled =
    (settings?.enableLowPowerMode ?? false) || debugControls.forceLowPower;
  const lowPowerModeEnabled =
    manualLowPowerModeEnabled || isWindowLowPower;
  const systemHighLoadDegrade =
    (snapshot?.cpuUsagePercent ?? 0) >= systemHighLoadCpuThreshold ||
    (snapshot?.memoryUsagePercent ?? 0) >= systemHighLoadMemoryThreshold;

  const handleDebugControlsChange = useCallback(
    (next: Partial<CoCatDebugControls>) => {
      setDebugControls((current) => ({ ...current, ...next }));
    },
    [],
  );

  const handlePerformanceReport = useCallback(
    (report: CoCatPerformanceReport) => {
      setPerformanceReport(report);
    },
    [],
  );

  const closeContextMenu = useCallback(() => {
    setIsMenuOpen(false);
  }, []);

  const runContextMenuAction = useCallback((action: () => void) => {
    setIsMenuOpen(false);
    action();
  }, []);

  const getInteractionDurationMs = useCallback(
    (state: CoCatAnimationState) => {
      if (state === "dataSorting") {
        return 5000;
      }

      return getSpriteSheetOneShotDurationMs(state) ?? getCoCatOneShotDurationMs(state);
    },
    [],
  );

  function playInteractionState(state: CoCatAnimationState) {
    const durationMs = getInteractionDurationMs(state);
    const requestId = ++interactionRequestIdRef.current;
    setInteractionRequest({
      requestId,
      state,
    });
    recordCoCatAnimationSeen(state, `interaction:${requestId}`);

    if (durationMs != null) {
      interactionTimerRef.current = window.setTimeout(
        () => finishInteractionRef.current?.(),
        durationMs + 80,
      );
    }
  }

  function recordCoCatAnimationSeen(
    animationState: CoCatAnimationState,
    reason: string,
  ) {
    const occurredAt = Date.now();
    void trackAchievementEvent({
      eventName: "cocat.animation_seen",
      occurredAt,
      idempotencyKey: `cocat.animation_seen:${animationState}:${reason}:${occurredAt}`,
      payload: { animationState, reason },
      source: "pet-window",
    }).catch((error) => {
      console.error("Failed to track CoCat animation achievement event", error);
    });
  }

  // finishInteractionRef：每次渲染都更新，确保始终捕获最新的 state setter 和 refs
  finishInteractionRef.current = () => {
    interactionTimerRef.current = null;

    // 检查队列，如果有下一个等待的 one-shot，接着播放
    const pending = pendingInteractionStateRef.current;
    if (pending != null) {
      pendingInteractionStateRef.current = null;
      playInteractionState(pending);
      return;
    }

    setInteractionRequest(null);
  };

  const triggerInteractionState = useCallback(
    (state: CoCatAnimationState) => {
      const durationMs = getInteractionDurationMs(state);

      if (interactionTimerRef.current != null) {
        if (durationMs != null) {
          // 当前有 one-shot 正在播放，且新请求也是 one-shot → 排队等待
          // 只保留最新的一个请求（后来的覆盖先来的）
          pendingInteractionStateRef.current = state;
          return;
        }
        // 新请求不是 one-shot（永久状态）→ 打断当前动画立即切换
        window.clearTimeout(interactionTimerRef.current);
        interactionTimerRef.current = null;
      }

      pendingInteractionStateRef.current = null;
      playInteractionState(state);
    },
    [getInteractionDurationMs],
  );

  const handleDebugInfo = useCallback((info: CoCatDebugInfo) => {
    debugInfoRef.current = info;
    const now = performance.now();
    const stateKey = `${info.animationState}:${info.activeVfxCount}:${info.transitionDurationMs}`;
    if (debugInfoStateRef.current === stateKey && now - debugInfoTimerRef.current < 250) {
      return;
    }

    debugInfoStateRef.current = stateKey;
    debugInfoTimerRef.current = now;
    setDebugInfo(info);
  }, []);

  const stopQaRun = useCallback(() => {
    qaCancelledRef.current = true;
    setDebugControls((current) => ({ ...current, stateOverride: null }));
    setQaRun((current) => ({
      ...current,
      isRunning: false,
      stuckState: current.stuckState,
    }));
  }, []);

  const runQaSequence = useCallback(
    (sequenceId: CoCatQaSequenceId) => {
      const sequence = getCoCatQaSequence(sequenceId);
      qaCancelledRef.current = false;
      setDebugControls((current) => ({ ...current, stateOverride: null }));
      setQaRun({
        cleanupOk: null,
        isRunning: true,
        sequenceId,
        stuckState: null,
        steps: sequence.states.map((state) => ({
          note: "queued",
          state,
          status: "pending",
        })),
      });

      void (async () => {
        for (const [index, state] of sequence.states.entries()) {
          if (qaCancelledRef.current) {
            return;
          }

          setQaRun((current) => ({
            ...current,
            steps: updateQaStep(current.steps, index, {
              note: "running",
              status: "running",
            }),
          }));
          setDebugControls((current) => ({ ...current, stateOverride: state }));

          await wait(160);
          const enteredState = debugInfoRef.current?.animationState;
          const entered = enteredState === state;
          await wait(getCoCatQaStepHoldMs(state));
          const currentState = debugInfoRef.current?.animationState;
          const shouldFallback = shouldCoCatQaStepAutoFallback(state);
          const fallbackOk = !shouldFallback || currentState !== state;
          const status = entered && fallbackOk ? "passed" : "failed";
          const note = entered
            ? shouldFallback
              ? fallbackOk
                ? "auto fallback ok"
                : "stuck"
              : "held ok"
            : `missed ${enteredState ?? "unknown"}`;

          setQaRun((current) => ({
            ...current,
            stuckState: status === "failed" ? state : current.stuckState,
            steps: updateQaStep(current.steps, index, { note, status }),
          }));
        }

        setDebugControls((current) => ({ ...current, stateOverride: null }));
        await wait(500);

        const cleanupOk =
          (debugInfoRef.current?.vfxParticleCount ?? 0) <= 1 &&
          (debugInfoRef.current?.activeVfxCount ?? 0) <= 1;
        setQaRun((current) => ({
          ...current,
          cleanupOk,
          isRunning: false,
          stuckState: cleanupOk ? current.stuckState : "idle",
        }));
      })();
    },
    [],
  );

  useEffect(() => {
    const animationState =
      lowPowerModeEnabled || staticModeEnabled
        ? "lowPowerStatic"
        : mapCatStateToCoCatState(visualState);
    if (reportedVisualAnimationRef.current === animationState) {
      return;
    }

    reportedVisualAnimationRef.current = animationState;
    recordCoCatAnimationSeen(animationState, "visual-state");
  }, [lowPowerModeEnabled, staticModeEnabled, visualState]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    // ── 全局 pointerup：在浏览器开发模式下可靠触发（Tauri 内也可能触发，作为快路径）
    function finalizeDrag() {
      if (!dragCandidateRef.current) {
        return;
      }

      // 清除 onMoved 防抖计时器，避免双重触发
      if (dropLandingTimerRef.current != null) {
        window.clearTimeout(dropLandingTimerRef.current);
        dropLandingTimerRef.current = null;
      }

      const didDrag = dragMovedRef.current;
      dragCandidateRef.current = false;
      dragMovedRef.current = false;
      setIsDragging(false);

      if (didDrag) {
        const endedAt = Date.now();
        const durationMs = Math.max(0, endedAt - dragStartAtRef.current);
        const distancePx = Math.round(dragDistanceRef.current);
        triggerInteractionState("dropLanding");
        void trackAchievementEvent({
          eventName: "pet.drag_end",
          occurredAt: endedAt,
          idempotencyKey: `pet.drag_end:${endedAt}:${distancePx}`,
          payload: { durationMs, distancePx },
          source: "pet-window",
        }).catch((error) => {
          console.error("Failed to track pet drag achievement event", error);
        });
      }
    }

    window.addEventListener("pointerup", finalizeDrag);

    if (!("__TAURI_INTERNALS__" in window)) {
      return () => {
        window.removeEventListener("pointerup", finalizeDrag);
      };
    }

    const currentWindow = getCurrentWindow();
    void currentWindow
      .setSize(
        new LogicalSize(
          compactPetWindowSize.width,
          compactPetWindowSize.height,
        ),
      )
      .catch((error) => {
        console.error("Failed to resize pet window:", error);
      });

    void currentWindow.onMoved((event) => {
      if (!dragCandidateRef.current) {
        return;
      }

      dragMovedRef.current = true;
      setIsMenuOpen(false);
      if (usePetStore.getState().isPanelOpen) {
        markPanelClosed();
        triggerInteractionState("panelClose");
      }
      void hidePetPanel();

      if (saveTimerRef.current != null) {
        window.clearTimeout(saveTimerRef.current);
      }

      saveTimerRef.current = window.setTimeout(() => {
        void saveWindowPosition("pet", event.payload.x, event.payload.y);
      }, 250);

      // ── Tauri 原生拖拽防抖检测：onMoved 停止触发 120ms 后视为拖拽结束。
      // OS 模态拖拽循环会屏蔽 WebView 的 pointerup，所以这是 Tauri 内唯一
      // 可靠的拖拽释放检测方式。finalizeDrag() 内置防重复触发守卫。
      if (dropLandingTimerRef.current != null) {
        window.clearTimeout(dropLandingTimerRef.current);
      }
      dropLandingTimerRef.current = window.setTimeout(() => {
        dropLandingTimerRef.current = null;
        if (!disposed) {
          finalizeDrag();
        }
      }, 120);
    }).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("pointerup", finalizeDrag);
      if (dropLandingTimerRef.current != null) {
        window.clearTimeout(dropLandingTimerRef.current);
      }
      if (interactionTimerRef.current != null) {
        window.clearTimeout(interactionTimerRef.current);
      }
      if (windowLowPowerTimerRef.current != null) {
        window.clearTimeout(windowLowPowerTimerRef.current);
      }
      if (saveTimerRef.current != null) {
        window.clearTimeout(saveTimerRef.current);
      }
      if (noddingTimerRef.current != null) {
        window.clearTimeout(noddingTimerRef.current);
      }
    };
  }, []);

  useEffect(() => {
    function clearWindowLowPowerTimer() {
      if (windowLowPowerTimerRef.current != null) {
        window.clearTimeout(windowLowPowerTimerRef.current);
        windowLowPowerTimerRef.current = null;
      }
    }

    function handleVisibilityChange() {
      clearWindowLowPowerTimer();
      setIsWindowLowPower(document.visibilityState === "hidden");
    }

    function handleBlur() {
      clearWindowLowPowerTimer();
      windowLowPowerTimerRef.current = window.setTimeout(() => {
        windowLowPowerTimerRef.current = null;
        setIsWindowLowPower(true);
      }, focusLowPowerDelayMs);
    }

    function handleFocus() {
      clearWindowLowPowerTimer();
      if (document.visibilityState !== "hidden") {
        setIsWindowLowPower(false);
      }
    }

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("blur", handleBlur);
    window.addEventListener("focus", handleFocus);

    return () => {
      clearWindowLowPowerTimer();
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("blur", handleBlur);
      window.removeEventListener("focus", handleFocus);
    };
  }, []);

  useEffect(() => {
    const bootWakeKey = "cocat.bootWake.played";

    try {
      if (window.sessionStorage.getItem(bootWakeKey) === "1") {
        return;
      }

      window.sessionStorage.setItem(bootWakeKey, "1");
    } catch {
      // Storage may be unavailable in constrained WebViews; the animation can still run.
    }

    triggerInteractionState("bootWake");
  }, [triggerInteractionState]);

  useEffect(() => {
    const cpu = snapshot?.cpuUsagePercent;
    const threshold = settings?.errorGlitchCpuThreshold ?? 96;

    if (cpu == null) {
      errorGlitchArmedRef.current = true;
      return;
    }

    if (cpu >= threshold && errorGlitchArmedRef.current) {
      errorGlitchArmedRef.current = false;
      triggerInteractionState("errorGlitch");
      return;
    }

    if (cpu < Math.max(0, threshold - 6)) {
      errorGlitchArmedRef.current = true;
    }
  }, [
    settings?.errorGlitchCpuThreshold,
    snapshot?.cpuUsagePercent,
    triggerInteractionState,
  ]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    if (!("__TAURI_INTERNALS__" in window)) {
      return undefined;
    }

    void listen<CoCatAnimationState>(
      "cocat:interaction-state",
      (event) => {
        if (event.payload === "panelOpen") {
          if (!usePetStore.getState().isPanelOpen) {
            markPanelOpen();
          }
        }

        if (event.payload === "panelClose") {
          if (usePetStore.getState().isPanelOpen) {
            markPanelClosed();
          }
        }

        triggerInteractionState(event.payload);
      },
    ).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [markPanelClosed, markPanelOpen, triggerInteractionState]);

  useEffect(() => {
    const previousState = previousStateRef.current;
    previousStateRef.current = visualState;

    if (
      warningStates.includes(visualState) &&
      previousState === "Idle"
    ) {
      setIsAlerting(true);
      playAudioFeedback("alert", settings?.enableSound ?? false);
      window.setTimeout(() => setIsAlerting(false), 240);
    }
  }, [settings?.enableSound, visualState]);

  useEffect(() => {
    if (!bubbleEnabled || !catMessage || catState === "Hidden") {
      setBubbleVisible(false);
      return undefined;
    }

    setBubbleVisible(true);
    const timer = window.setTimeout(
      () => setBubbleVisible(false), Math.max(3200, rewardMessageUntil - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [bubbleEnabled, catMessage, catState, rewardMessageUntil]);

  function handleClick() {
    if (isMenuOpen) {
      setIsMenuOpen(false);
      return;
    }

    if (dragMovedRef.current) {
      dragMovedRef.current = false;
      return;
    }

    const occurredAt = Date.now();
    const previousBurst = clickBurstRef.current;
    const withinBurst = occurredAt - previousBurst.startedAt <= 2000;
    const nextBurst = withinBurst
      ? { count: previousBurst.count + 1, startedAt: previousBurst.startedAt }
      : { count: 1, startedAt: occurredAt };
    if (nextBurst.count >= 3) {
      clickBurstRef.current = { count: 0, startedAt: 0 };
      void trackAchievementEvent({
        eventName: "pet.click_burst",
        occurredAt,
        idempotencyKey: `pet.click_burst:${occurredAt}`,
        payload: { clicks: nextBurst.count, windowMs: occurredAt - nextBurst.startedAt },
        source: "pet-window",
      }).catch((error) => {
        console.error("Failed to track pet click burst achievement event", error);
      });
    } else {
      clickBurstRef.current = nextBurst;
    }

    if (noddingTimerRef.current != null) {
      window.clearTimeout(noddingTimerRef.current);
    }
    setIsNodding(true);
    playAudioFeedback("click", settings?.enableSound ?? false);
    noddingTimerRef.current = window.setTimeout(() => {
      noddingTimerRef.current = null;
      setIsNodding(false);
    }, 80);
  }

  function handleDoubleClick() {
    dragMovedRef.current = false;
    setIsMenuOpen(false);
    playAudioFeedback("meow", settings?.enableSound ?? false);
    if (noddingTimerRef.current != null) {
      window.clearTimeout(noddingTimerRef.current);
    }
    setIsNodding(true);
    noddingTimerRef.current = window.setTimeout(() => {
      noddingTimerRef.current = null;
      setIsNodding(false);
    }, 80);
  }

  function triggerRewardAction(action: "pet" | "sortParts") {
    void rewardCoCatInteraction(action).catch((error) => {
      console.error("Failed to reward CoCat interaction:", error);
    });
  }

  function handlePointerMove(event: MouseEvent<HTMLDivElement>) {
    const hoverHitArea = resolveCoCatHoverHitArea(
      event.clientX,
      event.clientY,
      petCanvasRef.current?.getBoundingClientRect() ?? null,
    );

    setIsHovered(hoverHitArea.isInside);

    if (
      staticModeEnabled ||
      isDragging ||
      !hoverHitArea.isInside
    ) {
      setMouseOffset({ x: 0, y: 0 });
      return;
    }

    setMouseOffset({
      x: Number(
        (
          hoverHitArea.x * CORE_CAT_ANIMATION_CONFIG.hover.pointerInputMax
        ).toFixed(2),
      ),
      y: Number(
        (
          hoverHitArea.y * CORE_CAT_ANIMATION_CONFIG.hover.pointerInputMax
        ).toFixed(2),
      ),
    });
  }

  return (
    <div
      className="cwp-transparent-root cwp-pet-window-root"
      onMouseLeave={() => {
        setIsHovered(false);
        setMouseOffset({ x: 0, y: 0 });
      }}
      onMouseMove={handlePointerMove}
      onPointerDown={(event) => {
        if (!isMenuOpen) {
          return;
        }

        const target = event.target as HTMLElement;
        if (
          target.closest(".cwp-context-menu") ||
          target.closest(".cwp-pet-container")
        ) {
          return;
        }

        closeContextMenu();
      }}
    >
      <div className="cwp-pet-stage">
        <div
          aria-label="CoCat"
          className={`cwp-pet-container ${isDragging ? "is-dragging" : ""} ${
            staticModeEnabled ? "is-static-mode" : ""
          } ${lowPowerModeEnabled ? "is-low-power-mode" : ""}`}
          onClick={handleClick}
          onContextMenu={(event) => {
            event.preventDefault();
            dragCandidateRef.current = false;
            dragMovedRef.current = false;
            setIsMenuOpen((value) => !value);
          }}
          onDoubleClick={handleDoubleClick}
          onMouseDown={(event) => {
            if (event.button === 0) {
              dragCandidateRef.current = true;
              dragMovedRef.current = false;
              dragStartRef.current = { x: event.screenX, y: event.screenY };
              dragStartAtRef.current = Date.now();
              dragDistanceRef.current = 0;
              setIsMenuOpen(false);
            }
          }}
          onMouseMove={(event) => {
            if (!dragCandidateRef.current || event.buttons !== 1) {
              return;
            }

            const deltaX = Math.abs(event.screenX - dragStartRef.current.x);
            const deltaY = Math.abs(event.screenY - dragStartRef.current.y);
            dragDistanceRef.current = Math.max(
              dragDistanceRef.current,
              Math.hypot(deltaX, deltaY),
            );
            if (
              !dragMovedRef.current &&
              (deltaX > dragStartThresholdPx || deltaY > dragStartThresholdPx)
            ) {
              dragMovedRef.current = true;
              setIsDragging(true);
              triggerInteractionState("dragging");
              void startDraggingCurrentWindow();
            }
          }}
          // onMouseUp 在 Tauri 原生拖拽期间不可靠，已改为全局 pointerup 监听
          role="button"
          style={{
            opacity,
            transform: `scale(${scale})`,
            transformOrigin: "bottom center",
          }}
          tabIndex={0}
        >
          <PetSpeechBubble visible={bubbleVisible}>
            {catMessage || "CoCat 正在守护你的工作站。"}
          </PetSpeechBubble>

          <div className="cwp-pet-sprite-canvas" ref={petCanvasRef}>
            <CoCat
              catState={visualState}
              degradeVfx={systemHighLoadDegrade}
              debugPauseVfx={debugControls.pauseVfx}
              debugShowBounds={debugControls.showSkeletonBounds}
              debugShowVfxAnchors={debugControls.showVfxAnchors}
              debugStateOverride={debugControls.stateOverride}
              debugUpdateProgress={debugControls.updateProgress}
              interactionStateOverride={interactionRequest?.state ?? null}
              interactionStateRequestId={interactionRequest?.requestId ?? 0}
              isClicking={isNodding}
              isDragging={isDragging}
              lowPowerMode={lowPowerModeEnabled}
              onDebugInfo={import.meta.env.DEV ? handleDebugInfo : undefined}
              onPerformanceReport={
                import.meta.env.DEV ? handlePerformanceReport : undefined
              }
              pointerInside={isHovered}
              pointerOffset={mouseOffset}
              staticMode={staticModeEnabled}
            />
            <span
              className={`cwp-status-light ${statusLightClass(catState)} ${
                isAlerting ? "is-alerting" : ""
              }`}
            />
          </div>
        </div>

        <ContextMenu
          items={[
            {
              key: "main",
              label: "打开工坊",
              onClick: () => runContextMenuAction(() => void showMainRoute("workshop")),
            },
            {
              key: "settings",
              label: "打开设置",
              onClick: () => runContextMenuAction(() => void showMainRoute("settings")),
            },
            {
              key: "panel",
              label: "打开面板",
              onClick: () => runContextMenuAction(() => void showPetPanel()),
            },
            {
              key: "pet",
              label: "抚摸猫咪",
              onClick: () => runContextMenuAction(() => triggerRewardAction("pet")),
            },
            {
              key: "sort-parts",
              label: "整理零件",
              onClick: () => runContextMenuAction(() => triggerRewardAction("sortParts")),
            },
            {
              key: "release-memory",
              label: "释放内存",
              onClick: () => runContextMenuAction(() => void triggerMemoryRelease()),
            },
            {
              key: "hide",
              label: "隐藏 coCat",
              danger: true,
              onClick: () => runContextMenuAction(() => void hidePetWindow()),
            },
          ]}
          open={isMenuOpen}
        />
        {import.meta.env.DEV ? (
          <>
            <CoCatDebugPanel
              controls={debugControls}
              info={debugInfo}
              onChange={handleDebugControlsChange}
            />
            <CoCatPerformancePanel report={performanceReport} />
            <CoCatQaPanel
              onRun={runQaSequence}
              onStop={stopQaRun}
              run={qaRun}
            />
            <CoCatAssetPanel report={coCatRuntimeAssetReport} />
          </>
        ) : null}
      </div>
    </div>
  );
}

function statusLightClass(state: CatState) {
  if (state === "RepairHeavy" || state === "TemperatureCheck") {
    return "is-red";
  }

  if (state === "RepairLight" || state === "MemoryCrowded") {
    return "is-orange";
  }

  return "";
}

function updateQaStep(
  steps: CoCatQaRunState["steps"],
  index: number,
  patch: Partial<CoCatQaRunState["steps"][number]>,
) {
  return steps.map((step, currentIndex) =>
    currentIndex === index ? { ...step, ...patch } : step,
  );
}

function wait(durationMs: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, durationMs);
  });
}
