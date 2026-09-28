import { useEffect, useMemo, useState } from "react";
import type { FocusSession } from "../../types/focus";
import { useFocusStore } from "../../stores/focusStore";
import { PixelIcon } from "../../ui/PixelIcon";
import { creditedSeconds } from "../../types/rewards";

const DURATION_OPTIONS = [25, 50, 90] as const;

export function FocusPage() {
  const book = useFocusStore((state) => state.book);
  const activeSession = useFocusStore((state) => state.activeSession);
  const isLoading = useFocusStore((state) => state.isLoading);
  const isStarting = useFocusStore((state) => state.isStarting);
  const isEnding = useFocusStore((state) => state.isEnding);
  const loadError = useFocusStore((state) => state.loadError);
  const load = useFocusStore((state) => state.load);
  const start = useFocusStore((state) => state.start);
  const complete = useFocusStore((state) => state.complete);
  const abandon = useFocusStore((state) => state.abandon);
  const [taskLabel, setTaskLabel] = useState("");
  const [durationMinutes, setDurationMinutes] = useState<number>(25);
  const [now, setNow] = useState(Date.now());
  const [actionError, setActionError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeSession) {
      return undefined;
    }
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [activeSession]);

  const history = useMemo(
    () =>
      [...(book?.sessions ?? [])]
        .filter((session) => session.status !== "active")
        .sort((left, right) => (right.endedAt ?? right.startedAt) - (left.endedAt ?? left.startedAt))
        .slice(0, 8),
    [book],
  );
  const todaySummary = useMemo(() => summarizeToday(book?.sessions ?? []), [book]);
  const timing = activeSession ? getSessionTiming(activeSession, now) : null;

  async function handleStart() {
    setActionError(null);
    try {
      await start(taskLabel, durationMinutes);
      setTaskLabel("");
    } catch (error) {
      setActionError(errorMessage(error, "无法开始专注会话"));
    }
  }

  async function handleEnd(action: "complete" | "abandon") {
    setActionError(null);
    try {
      await (action === "complete" ? complete() : abandon());
    } catch (error) {
      setActionError(errorMessage(error, "无法更新专注会话"));
    }
  }

  return (
    <div className="cwp-page cwp-focus-page">
      <div className="cwp-page-header">
        <div className="cwp-page-title">
          <h2>专注舱</h2>
          <p>FOCUS SESSION</p>
        </div>
        <div className={`cwp-focus-live-badge${activeSession ? " is-active" : ""}`}>
          <span aria-hidden="true" />
          {activeSession
            ? `工坊 ×${activeSession.productionMultiplier.toFixed(1)}`
            : "等待任务"}
        </div>
      </div>

      <div className="cwp-focus-content">
        <section className="cwp-focus-console" aria-label="当前专注会话">
          {activeSession && timing ? (
            <div className="cwp-focus-active-session">
              <div className="cwp-focus-task-row">
                <PixelIcon name="focus" size={20} />
                <div>
                  <span>当前任务</span>
                  <strong title={activeSession.taskLabel}>{activeSession.taskLabel}</strong>
                </div>
              </div>

              <time className="cwp-focus-clock" dateTime={`PT${timing.remainingSeconds}S`}>
                {formatCountdown(timing.remainingSeconds)}
              </time>
              <div className="cwp-focus-progress" aria-label={`专注进度 ${timing.progressPercent}%`}>
                <span style={{ width: `${timing.progressPercent}%` }} />
              </div>

              <div className="cwp-focus-session-stats">
                <div>
                  <span>已投入</span>
                  <strong>{formatCompactDuration(timing.elapsedSeconds)}</strong>
                </div>
                <div>
                  <span>分心</span>
                  <strong>{activeSession.distractionCount} 次</strong>
                </div>
                <div>
                  <span>生产</span>
                  <strong>×{activeSession.productionMultiplier.toFixed(1)}</strong>
                </div>
              </div>

              <div className="cwp-focus-actions">
                <button
                  className="cwp-compact-button is-primary"
                  disabled={isEnding}
                  onClick={() => void handleEnd("complete")}
                  type="button"
                >
                  {isEnding ? "处理中..." : "提前结束并结算"}
                </button>
                <button
                  className="cwp-compact-button is-danger"
                  disabled={isEnding}
                  onClick={() => void handleEnd("abandon")}
                  type="button"
                >
                  放弃本次
                </button>
              </div>
            </div>
          ) : (
            <form
              className="cwp-focus-setup"
              onSubmit={(event) => {
                event.preventDefault();
                void handleStart();
              }}
            >
              <div className="cwp-focus-setup-heading">
                <PixelIcon name="focus" size={18} />
                <div>
                  <strong>交付一个任务</strong>
                  <span>CoCat 已就位</span>
                </div>
              </div>
              <label htmlFor="focus-task">本次任务</label>
              <input
                autoComplete="off"
                id="focus-task"
                maxLength={40}
                onChange={(event) => setTaskLabel(event.target.value)}
                placeholder="例如：完成项目周报"
                type="text"
                value={taskLabel}
              />
              <fieldset>
                <legend>专注时长</legend>
                <div className="cwp-focus-duration-options">
                  {DURATION_OPTIONS.map((minutes) => (
                    <button
                      aria-pressed={durationMinutes === minutes}
                      className={durationMinutes === minutes ? "is-active" : ""}
                      key={minutes}
                      onClick={() => setDurationMinutes(minutes)}
                      type="button"
                    >
                      {minutes} 分
                    </button>
                  ))}
                </div>
              </fieldset>
              <button
                className="cwp-compact-button is-primary cwp-focus-start"
                disabled={!taskLabel.trim() || isStarting}
                type="submit"
              >
                {isStarting ? "正在启动..." : "开始专注"}
              </button>
            </form>
          )}
          {actionError ? (
            <div className="cwp-focus-error" role="alert">
              {actionError}
            </div>
          ) : null}
        </section>

        <section className="cwp-focus-records" aria-label="专注记录">
          <div className="cwp-focus-today">
            <div className="cwp-focus-section-title">
              <strong>今日专注</strong>
              <span>{todaySummary.completedCount} 次完成</span>
            </div>
            <div className="cwp-focus-today-stats">
              <div>
                <strong>{todaySummary.totalMinutes}</strong>
                <span>分钟</span>
              </div>
              <div>
                <strong>{todaySummary.averageQuality}%</strong>
                <span>质量</span>
              </div>
              <div>
                <strong>{todaySummary.distractionCount}</strong>
                <span>分心</span>
              </div>
            </div>
          </div>

          <div className="cwp-focus-history-heading">
            <strong>最近记录</strong>
            <span>
              {book?.sessions.filter((session) => session.status !== "active").length ?? 0} 次累计
            </span>
          </div>

          {isLoading && !book ? (
            <div className="cwp-focus-loading" aria-label="正在加载专注记录">
              <span />
              <span />
              <span />
            </div>
          ) : loadError && !book ? (
            <div className="cwp-focus-empty" role="alert">
              <strong>记录加载失败</strong>
              <span>{loadError}</span>
              <button className="cwp-compact-button" onClick={() => void load()} type="button">
                重试
              </button>
            </div>
          ) : history.length > 0 ? (
            <div className="cwp-focus-history-list">
              {history.map((session) => (
                <HistoryRow key={session.id} session={session} />
              ))}
            </div>
          ) : (
            <div className="cwp-focus-empty">
              <PixelIcon name="focus" size={20} />
              <strong>还没有专注记录</strong>
              <span>完成第一项任务后会出现在这里。</span>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function HistoryRow({ session }: { session: FocusSession }) {
  const durationSeconds = creditedSeconds(session);
  const completed = session.status === "completed";

  return (
    <article className="cwp-focus-history-row">
      <span className={`cwp-focus-history-state ${completed ? "is-completed" : "is-abandoned"}`}>
        {completed ? (session.rewardVersion > 0 && durationSeconds < session.plannedDurationSeconds ? "提前" : "完成") : "中止"}
      </span>
      <div className="cwp-focus-history-task">
        <strong title={session.taskLabel}>{session.taskLabel}</strong>
        <span>{formatSessionTime(session.endedAt ?? session.startedAt)}</span>
      </div>
      <div className="cwp-focus-history-result">
        <strong>{formatCompactDuration(durationSeconds)}</strong>
        <span>{completed ? `质量 ${Math.round(session.focusQuality * 100)}%` : "未结算"}</span>
      </div>
    </article>
  );
}

function getSessionTiming(session: FocusSession, now: number) {
  const elapsedSeconds = creditedSeconds(session, now);
  const remainingSeconds = Math.max(0, session.plannedDurationSeconds - elapsedSeconds);
  const progressPercent = Math.min(
    100,
    Math.round((elapsedSeconds / Math.max(1, session.plannedDurationSeconds)) * 100),
  );
  return { elapsedSeconds, remainingSeconds, progressPercent };
}

function summarizeToday(sessions: FocusSession[]) {
  const start = new Date();
  start.setHours(0, 0, 0, 0);
  const completed = sessions.filter(
    (session) => session.status === "completed" && (session.endedAt ?? 0) >= start.getTime(),
  );
  const totalSeconds = completed.reduce(
    (sum, session) =>
      sum +
      creditedSeconds(session),
    0,
  );
  const qualityTotal = completed.reduce((sum, session) => sum + session.focusQuality, 0);
  return {
    completedCount: completed.length,
    totalMinutes: Math.floor(totalSeconds / 60),
    averageQuality: completed.length ? Math.round((qualityTotal / completed.length) * 100) : 0,
    distractionCount: completed.reduce((sum, session) => sum + session.distractionCount, 0),
  };
}

function formatCountdown(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function formatCompactDuration(seconds: number) {
  if (seconds < 60) {
    return `${seconds} 秒`;
  }
  return `${Math.floor(seconds / 60)} 分`;
}

function formatSessionTime(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(timestamp);
}

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return typeof error === "string" && error ? error : fallback;
}
