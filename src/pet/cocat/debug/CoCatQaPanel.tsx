import type { CoCatAnimationState } from "../animation/animationTypes";
import type { CoCatQaSequenceId } from "./coCatQaSequences";
import { CORE_CAT_QA_SEQUENCES } from "./coCatQaSequences";

export type CoCatQaStepStatus =
  | "pending"
  | "running"
  | "passed"
  | "warning"
  | "failed";

export interface CoCatQaStepResult {
  note: string;
  state: CoCatAnimationState;
  status: CoCatQaStepStatus;
}

export interface CoCatQaRunState {
  cleanupOk: boolean | null;
  isRunning: boolean;
  sequenceId: CoCatQaSequenceId | null;
  stuckState: CoCatAnimationState | null;
  steps: CoCatQaStepResult[];
}

export function CoCatQaPanel({
  run,
  onRun,
  onStop,
}: {
  run: CoCatQaRunState;
  onRun: (id: CoCatQaSequenceId) => void;
  onStop: () => void;
}) {
  if (!import.meta.env.DEV) {
    return null;
  }

  return (
    <aside className="cocat-qa-panel">
      <header>
        <strong>CoCat QA</strong>
        {run.isRunning ? (
          <button onClick={onStop} type="button">
            Stop
          </button>
        ) : null}
      </header>

      <div className="cocat-qa-actions">
        {Object.values(CORE_CAT_QA_SEQUENCES).map((sequence) => (
          <button
            disabled={run.isRunning}
            key={sequence.id}
            onClick={() => onRun(sequence.id)}
            type="button"
          >
            {sequence.label}
          </button>
        ))}
      </div>

      <dl>
        <div>
          <dt>Sequence</dt>
          <dd>{run.sequenceId ?? "idle"}</dd>
        </div>
        <div>
          <dt>Stuck</dt>
          <dd>{run.stuckState ?? "none"}</dd>
        </div>
        <div>
          <dt>VFX Clean</dt>
          <dd>
            {run.cleanupOk == null ? "-" : run.cleanupOk ? "ok" : "blocked"}
          </dd>
        </div>
      </dl>

      <ol>
        {run.steps.length > 0 ? (
          run.steps.map((step, index) => (
            <li className={`is-${step.status}`} key={`${step.state}-${index}`}>
              <span>{step.state}</span>
              <small>{step.note}</small>
            </li>
          ))
        ) : (
          <li className="is-empty">No QA run yet</li>
        )}
      </ol>
    </aside>
  );
}
