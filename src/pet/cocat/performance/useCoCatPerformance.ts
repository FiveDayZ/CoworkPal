import { useEffect, useRef, useState } from "react";
import type {
  CoCatPerformanceFrameInput,
  CoCatPerformanceReport,
} from "./coCatPerformanceTypes";
import {
  createCoCatPerformanceMonitor,
  createInitialCoCatPerformanceReport,
} from "./coCatPerformanceMonitor";

export function useCoCatPerformance(input: CoCatPerformanceFrameInput) {
  const monitorRef = useRef<ReturnType<
    typeof createCoCatPerformanceMonitor
  > | null>(null);
  const [report, setReport] = useState<CoCatPerformanceReport>(() =>
    createInitialCoCatPerformanceReport(input),
  );

  if (monitorRef.current == null) {
    monitorRef.current = createCoCatPerformanceMonitor(input);
  }

  useEffect(() => {
    const nextReport = monitorRef.current?.recordFrame(input);
    if (nextReport) {
      setReport(nextReport);
    }
  }, [
    input.activeVfxCount,
    input.animationState,
    input.isLowPower,
    input.isVfxAutoDegraded,
    input.isVfxPaused,
    input.previousAnimationState,
    input.reducedMotion,
    input.transitionDurationMs,
    input.vfxParticleCount,
  ]);

  return report;
}
