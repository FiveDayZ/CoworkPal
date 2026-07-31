import type { CoCatPerformanceReport } from "./coCatPerformanceTypes";

export function CoCatPerformancePanel({
  report,
}: {
  report: CoCatPerformanceReport | null;
}) {
  if (!import.meta.env.DEV || !report) {
    return null;
  }

  return (
    <aside className="cocat-performance-panel">
      <header>
        <strong>CoCat Perf</strong>
        <span>{report.isVfxAutoDegraded ? "degraded" : "normal"}</span>
      </header>

      <dl>
        <div>
          <dt>FPS</dt>
          <dd>{report.fps}</dd>
        </div>
        <div>
          <dt>Frame</dt>
          <dd>{report.averageFrameMs.toFixed(1)}ms</dd>
        </div>
        <div>
          <dt>State</dt>
          <dd>{report.animationState}</dd>
        </div>
        <div>
          <dt>Prev</dt>
          <dd>{report.previousAnimationState}</dd>
        </div>
        <div>
          <dt>Transition</dt>
          <dd>{report.transitionDurationMs}ms</dd>
        </div>
        <div>
          <dt>VFX</dt>
          <dd>
            {report.activeVfxCount}/{report.vfxParticleCount}
          </dd>
        </div>
        <div>
          <dt>LowPower</dt>
          <dd>{report.isLowPower ? "on" : "off"}</dd>
        </div>
        <div>
          <dt>Paused</dt>
          <dd>{report.isVfxPaused ? "yes" : "no"}</dd>
        </div>
        <div>
          <dt>Reduced</dt>
          <dd>{report.reducedMotion ? "yes" : "no"}</dd>
        </div>
      </dl>

      <ol>
        {report.stateTransitions.length > 0 ? (
          report.stateTransitions.map((item) => (
            <li key={`${item.atMs}:${item.from}:${item.to}`}>
              <span>{item.from}</span>
              <span>{item.to}</span>
              <small>{item.durationMs}ms</small>
            </li>
          ))
        ) : (
          <li className="is-empty">No transitions sampled</li>
        )}
      </ol>
    </aside>
  );
}
