import type {
  CoCatPerformanceFrameInput,
  CoCatPerformanceReport,
  CoCatStateTransitionRecord,
} from "./coCatPerformanceTypes";

export const COCAT_PERFORMANCE_SAMPLE_INTERVAL_MS = 500;
export const COCAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT = 10;

export function createInitialCoCatPerformanceReport(
  input: CoCatPerformanceFrameInput,
  nowMs = 0,
): CoCatPerformanceReport {
  return {
    ...input,
    averageFrameMs: 0,
    fps: 0,
    frameCount: 0,
    sampledAtMs: nowMs,
    stateTransitions: [],
  };
}

export function createCoCatPerformanceMonitor(
  initialInput: CoCatPerformanceFrameInput,
  nowMs = 0,
) {
  let lastFrameAtMs = nowMs;
  let lastSampleAtMs = nowMs;
  let framesSinceSample = 0;
  let accumulatedFrameMs = 0;
  let lastAnimationState = initialInput.animationState;
  let latestInput = initialInput;
  const stateTransitions: CoCatStateTransitionRecord[] = [];

  return {
    recordFrame(
      input: CoCatPerformanceFrameInput,
      now = performance.now(),
    ): CoCatPerformanceReport | null {
      latestInput = input;

      if (input.animationState !== lastAnimationState) {
        stateTransitions.unshift({
          atMs: now,
          durationMs: input.transitionDurationMs,
          from: lastAnimationState,
          to: input.animationState,
        });
        stateTransitions.length = Math.min(
          stateTransitions.length,
          COCAT_PERFORMANCE_TRANSITION_HISTORY_LIMIT,
        );
        lastAnimationState = input.animationState;
      }

      const frameDeltaMs = Math.max(0, now - lastFrameAtMs);
      lastFrameAtMs = now;
      framesSinceSample += 1;
      accumulatedFrameMs += frameDeltaMs;

      const sampleDeltaMs = now - lastSampleAtMs;
      if (sampleDeltaMs < COCAT_PERFORMANCE_SAMPLE_INTERVAL_MS) {
        return null;
      }

      const report: CoCatPerformanceReport = {
        ...latestInput,
        averageFrameMs:
          framesSinceSample > 0 ? accumulatedFrameMs / framesSinceSample : 0,
        fps:
          sampleDeltaMs > 0
            ? Math.round((framesSinceSample * 1000) / sampleDeltaMs)
            : 0,
        frameCount: framesSinceSample,
        sampledAtMs: now,
        stateTransitions: [...stateTransitions],
      };

      lastSampleAtMs = now;
      framesSinceSample = 0;
      accumulatedFrameMs = 0;

      return report;
    },
    snapshot(now = performance.now()): CoCatPerformanceReport {
      return {
        ...latestInput,
        averageFrameMs:
          framesSinceSample > 0 ? accumulatedFrameMs / framesSinceSample : 0,
        fps: 0,
        frameCount: framesSinceSample,
        sampledAtMs: now,
        stateTransitions: [...stateTransitions],
      };
    },
  };
}
