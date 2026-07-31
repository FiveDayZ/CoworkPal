import type { CoCatAnimationState } from "../animation/animationTypes";
import type { CoCatDebugInfo } from "../CoCat";

export interface CoCatDebugControls {
  forceLowPower: boolean;
  pauseVfx: boolean;
  showSkeletonBounds: boolean;
  showVfxAnchors: boolean;
  stateOverride: CoCatAnimationState | null;
  updateProgress: number;
}

interface DebugStateButton {
  label: string;
  state: CoCatAnimationState;
  status: "ready" | "experimental";
}

const debugStates: DebugStateButton[] = [
  { label: "Idle", state: "idle", status: "ready" },
  { label: "Hover", state: "hover", status: "ready" },
  { label: "Click", state: "click", status: "ready" },
  { label: "Temperature", state: "temperatureCheck", status: "ready" },
  { label: "Memory", state: "memoryCrowded", status: "ready" },
  { label: "Repair", state: "repairing", status: "ready" },
  { label: "Data", state: "dataSorting", status: "ready" },
  { label: "Scared", state: "scaredByMouse", status: "ready" },
  { label: "Fish", state: "eatingFish", status: "ready" },
  { label: "Pet", state: "pettingHearts", status: "ready" },
  { label: "Sleep", state: "sleep", status: "ready" },
  { label: "Low Power", state: "lowPowerStatic", status: "ready" },
  { label: "Celebrate", state: "celebrate", status: "ready" },
  { label: "Workshop Up", state: "workshopUpgrade", status: "ready" },
  { label: "Module Up", state: "moduleUpgrade", status: "ready" },
  { label: "Boot", state: "bootWake", status: "ready" },
  { label: "Drag", state: "dragging", status: "ready" },
  { label: "Drop", state: "dropLanding", status: "ready" },
  { label: "Panel Open", state: "panelOpen", status: "ready" },
  { label: "Panel Close", state: "panelClose", status: "ready" },
  { label: "Error", state: "errorGlitch", status: "ready" },
  { label: "Update", state: "updateInstalling", status: "ready" },
  { label: "Badge", state: "achievementPop", status: "ready" },
];

export function CoCatDebugPanel({
  controls,
  info,
  onChange,
}: {
  controls: CoCatDebugControls;
  info: CoCatDebugInfo | null;
  onChange: (next: Partial<CoCatDebugControls>) => void;
}) {
  if (!import.meta.env.DEV) {
    return null;
  }

  function triggerState(state: CoCatAnimationState) {
    if (controls.stateOverride !== state) {
      onChange({ stateOverride: state });
      return;
    }

    onChange({ stateOverride: null });
    window.setTimeout(() => onChange({ stateOverride: state }), 0);
  }

  return (
    <aside className="cocat-debug-panel">
      <header>
        <strong>CoCat Debug</strong>
        <button onClick={() => onChange({ stateOverride: null })} type="button">
          Auto
        </button>
      </header>

      <dl>
        <div>
          <dt>Current</dt>
          <dd>{info?.animationState ?? "unknown"}</dd>
        </div>
        <div>
          <dt>Previous</dt>
          <dd>{info?.previousAnimationState ?? "unknown"}</dd>
        </div>
        <div>
          <dt>Transition</dt>
          <dd>{info ? `${info.transitionDurationMs}ms` : "-"}</dd>
        </div>
        <div>
          <dt>VFX</dt>
          <dd>
            {info?.activeVfxCount ?? 0}/{info?.vfxParticleCount ?? 0}
          </dd>
        </div>
        <div>
          <dt>Elapsed</dt>
          <dd>{info ? `${Math.round(info.stateElapsedMs)}ms` : "-"}</dd>
        </div>
        <div>
          <dt>LowPower</dt>
          <dd>{info?.isLowPower ? "on" : "off"}</dd>
        </div>
        <div>
          <dt>VFX Safe</dt>
          <dd>{info?.isVfxAutoDegraded ? "degraded" : "normal"}</dd>
        </div>
      </dl>

      <div className="cocat-debug-switches">
        <label>
          <input
            checked={controls.pauseVfx}
            onChange={(event) => onChange({ pauseVfx: event.target.checked })}
            type="checkbox"
          />
          Pause VFX
        </label>
        <label>
          <input
            checked={controls.forceLowPower}
            onChange={(event) =>
              onChange({ forceLowPower: event.target.checked })
            }
            type="checkbox"
          />
          Force LowPower
        </label>
        <label>
          <input
            checked={controls.showSkeletonBounds}
            onChange={(event) =>
              onChange({ showSkeletonBounds: event.target.checked })
            }
            type="checkbox"
          />
          Bounds
        </label>
        <label>
          <input
            checked={controls.showVfxAnchors}
            onChange={(event) =>
              onChange({ showVfxAnchors: event.target.checked })
            }
            type="checkbox"
          />
          VFX Anchors
        </label>
        <label className="cocat-debug-progress">
          Update {Math.round(controls.updateProgress * 100)}%
          <input
            max={100}
            min={0}
            onChange={(event) =>
              onChange({ updateProgress: Number(event.target.value) / 100 })
            }
            type="range"
            value={Math.round(controls.updateProgress * 100)}
          />
        </label>
      </div>

      <div className="cocat-debug-state-grid">
        {debugStates.map((item) => (
          <button
            className={
              controls.stateOverride === item.state ? "is-active" : undefined
            }
            key={item.state}
            onClick={() => triggerState(item.state)}
            title={item.state}
            type="button"
          >
            {item.label}
          </button>
        ))}
      </div>
    </aside>
  );
}
