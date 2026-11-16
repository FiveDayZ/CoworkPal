import { useEffect, useRef, useState } from "react";
import type {
  CoCatPerformanceFrameInput,
  CoCatPerformanceReport,
} from "./coCatPerformanceTypes";
import {
  createCoCatPerformanceMonitor,
  createInitialCoCatPerformanceReport,
} from "./coCatPerformanceMonitor";

export function useCoCatPerformance(input: CoCatPerformanceFrameInput, sampledAtMs: number) {
  const monitorRef = useRef<ReturnType<
    typeof createCoCatPerformanceMonitor
  > | null>(null);
  const lastRecordedAt = useRef(0);
  const [report, setReport] = useState<CoCatPerformanceReport>(() =>
    createInitialCoCatPerformanceReport(input),
  );

  if (monitorRef.current == null) {
    monitorRef.current = createCoCatPerformanceMonitor(input, sampledAtMs);
  }

  useEffect(() => {
    if (sampledAtMs <= lastRecordedAt.current) return;
    lastRecordedAt.current = sampledAtMs;
    const nextReport = monitorRef.current?.recordFrame(input, sampledAtMs);
    if (nextReport) {
      setReport(nextReport);
    }
  }, [
    sampledAtMs,
  ]);

  return report;
}
