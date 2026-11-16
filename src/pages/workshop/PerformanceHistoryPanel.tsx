import { useEffect, useState } from "react";
import { getPerformanceHistory } from "../../services/tauriCommands";
import { formatBytes } from "../../services/formatters";
import { useUiStore } from "../../stores/uiStore";
import type { PerformanceHistory } from "../../types/performance";
const METRICS = ["CPU 负载", "内存占用", "CPU 温度", "GPU 温度"];
const time = (at: number) => new Date(at).toLocaleTimeString("zh-CN", { hour12: false });

export function PerformanceHistoryPanel() {
  const open = useUiStore((state) => state.performanceHistoryOpen);
  return <section className="cwp-performance-history">
    <button type="button" className="cwp-section-toggle" aria-expanded={open} onClick={() => useUiStore.getState().setPerformanceHistoryOpen(!open)}><strong>性能事件回放</strong><span>{open ? "收起" : "最近 10 分钟 · 展开"}</span></button>
    {open ? <HistoryContent /> : null}
  </section>;
}

function HistoryContent() {
  const [history, setHistory] = useState<PerformanceHistory | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [metric, setMetric] = useState(0);
  const [eventIndex, setEventIndex] = useState(0);
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    async function load() {
      if (!document.hidden) {
        try {
          const value = await getPerformanceHistory();
          if (!stopped) { setHistory(value); setError(null); }
        } catch (error) { if (!stopped) setError(String(error)); }
      }
      if (!stopped) timer = setTimeout(() => void load(), 5000);
    }
    void load();
    return () => { stopped = true; clearTimeout(timer); };
  }, [retry]);
  const samples = history?.samples ?? [];
  const first = samples[0]?.timestamp ?? 0;
  const span = Math.max(1, (samples.at(-1)?.timestamp ?? first) - first);
  const segments: string[] = [];
  let points: string[] = [];
  samples.forEach((sample, index) => {
    const value = sample.values[metric];
    if (value == null || (index > 0 && sample.timestamp - samples[index - 1].timestamp > 15000)) {
      if (points.length) segments.push(points.join(" "));
      points = [];
    }
    if (value != null) points.push(`${(sample.timestamp - first) / span * 600},${100 - Math.min(100, Math.max(0, value))}`);
  });
  if (points.length) segments.push(points.join(" "));
  const events = [...(history?.events ?? [])].reverse();
  const selectedIndex = Math.min(eventIndex, Math.max(0, events.length - 1));
  const event = events[selectedIndex];
  return <div className="cwp-performance-content">
    <p>本次运行 · 最近 10 分钟 · 连续越阈值 10 秒记为事件</p>
    {error ? <p role="alert">读取失败：{error} <button type="button" className="cwp-compact-button" onClick={() => setRetry((n) => n + 1)}>重试</button></p> : null}
    {!history && !error ? <p role="status">正在读取性能记录…</p> : null}
    <div className="cwp-inline-actions" aria-label="趋势指标">{METRICS.map((name, index) => <button type="button" className="cwp-compact-button" key={name} aria-pressed={metric === index} onClick={() => setMetric(index)}>{name}</button>)}</div>
    {samples.length ? <>
      <svg viewBox="0 0 600 110" role="img" aria-label={`${METRICS[metric]}最近十分钟趋势，纵轴 0 到 100${metric < 2 ? '%' : '°C'}`}>
        <path d="M0 0H600 M0 50H600 M0 100H600" stroke="currentColor" opacity="0.15" />
        {segments.map((points, index) => <polyline key={index} points={points} fill="none" stroke="var(--color-tech-cyan)" strokeWidth="2" />)}
      </svg><div className="cwp-history-axis"><span>{time(first)}</span><span>{time(samples.at(-1)!.timestamp)} · {metric < 2 ? '%' : '°C'}（0–100）</span></div>
    </> : <p>尚无采样记录，桌面应用开始采样后显示。</p>}
    {event ? <div className="cwp-performance-event">
      <div className="cwp-history-pagination">
        <strong>{METRICS[event.metric]} · {time(event.startedAt)}</strong>
        <button type="button" className="cwp-compact-button" aria-label="较新事件" disabled={selectedIndex === 0} onClick={() => setEventIndex(selectedIndex - 1)}>‹</button>
        <span>{selectedIndex + 1}/{events.length}</span>
        <button type="button" className="cwp-compact-button" aria-label="较早事件" disabled={selectedIndex === events.length - 1} onClick={() => setEventIndex(selectedIndex + 1)}>›</button>
      </div>
      <p>峰值 {event.peak.toFixed(1)}{event.metric < 2 ? '%' : '°C'} · 阈值 {event.threshold.toFixed(0)} · {Math.max(0, Math.floor(((event.endedAt ?? event.lastSeenAt) - event.startedAt) / 1000))} 秒 · {event.interrupted ? "采样中断" : event.endedAt ? "已恢复" : "持续中"}</p>
      <span className="cwp-process-context">同期占用，仅供排查：</span>
      {event.processes.length ? <ul>{event.processes.map((process) => <li key={process.pid} title={process.name}>{process.name} · CPU {process.cpuUsagePercent.toFixed(1)}% · {formatBytes(process.memoryBytes)}</li>)}</ul> : <p>无可用进程样本。</p>}
    </div> : history ? <p>最近没有持续性能事件。</p> : null}
  </div>;
}
