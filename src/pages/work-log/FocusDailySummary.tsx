import { useState } from "react";
import { useFocusStore } from "../../stores/focusStore";
import { useUiStore } from "../../stores/uiStore";
import { localDateKey } from "../../services/dateKeys";
import { creditedSeconds } from "../../types/rewards";

export function FocusDailySummary({ date }: { date: string }) {
  const book = useFocusStore((state) => state.book);
  const sessions = (book?.sessions ?? []).filter((session) => session.status !== "active" && localDateKey(session.endedAt ?? session.startedAt) === date);
  const [page, setPage] = useState(0);
  const currentPage = Math.min(page, Math.max(0, sessions.length - 1));
  const session = sessions[currentPage];
  return <section className="cwp-focus-daily" aria-label="任务复盘">
    <div className="cwp-section-title"><strong>任务复盘</strong><button type="button" className="cwp-compact-button" onClick={() => useUiStore.getState().setMainRoute("focus")}>前往专注舱</button></div>
    {session ? <article key={session.id}>
      <strong>{session.taskLabel}</strong><span> · {Math.floor(creditedSeconds(session) / 60)} 分钟 · {session.status === "completed" ? "已完成" : "已中止"}</span>
      <p>{session.outcome || "尚未记录结果，可在专注舱补写。"}</p>
    </article> : <p>这一天还没有已结束的专注记录。</p>}
    {sessions.length > 1 ? <div className="cwp-history-pagination">
      <button type="button" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>上一条</button>
      <span aria-live="polite">{currentPage + 1}/{sessions.length}</span>
      <button type="button" disabled={currentPage === sessions.length - 1} onClick={() => setPage(currentPage + 1)}>下一条</button>
    </div> : null}
  </section>;
}
