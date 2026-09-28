import { useEffect, useState } from "react";
import { useAchievementStore } from "../../stores/achievementStore";
import { useUiStore } from "../../stores/uiStore";
import { badgeAssets } from "../../ui/assets";
import { PixelIcon } from "../../ui/PixelIcon";

export function WeeklyGoalsCard() {
  const weeklyGoals = useAchievementStore((state) => state.weeklyGoals);
  const loadWeeklyGoals = useAchievementStore((state) => state.loadWeeklyGoals);
  const setRoute = useUiStore((state) => state.setMainRoute);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (weeklyGoals) {
      return;
    }
    void loadWeeklyGoals().catch((error) => {
      console.error("Failed to load weekly goals", error);
      setFailed(true);
    });
  }, [loadWeeklyGoals, weeklyGoals]);

  const completed = weeklyGoals?.goals.filter((goal) => goal.isComplete).length ?? 0;

  return (
    <section className="cwp-weekly-goals" aria-label="本周目标">
      <header>
        <div>
          <PixelIcon name="calendar" size={13} />
          <strong>本周目标</strong>
        </div>
        <span className="cwp-weekly-goals-count">
          {completed}/3
        </span>
      </header>

      {failed ? (
        <button
          className="cwp-weekly-goals-retry"
          onClick={() => {
            setFailed(false);
            void loadWeeklyGoals().catch(() => setFailed(true));
          }}
          type="button"
        >
          目标读取失败 · 重试
        </button>
      ) : null}

      {!weeklyGoals && !failed ? (
        <div className="cwp-weekly-goals-loading" aria-label="正在读取本周目标">
          <span />
          <span />
          <span />
        </div>
      ) : null}

      {weeklyGoals ? (
        <div className="cwp-weekly-goals-list">
          {weeklyGoals.goals.map((goal) => {
            const progress = goal.percent;
            return (
              <button
                className={goal.isComplete ? "is-complete" : ""}
                key={goal.goalId}
                onClick={() => setRoute(goal.routeKey)}
                title={goal.progressLabel}
                type="button"
              >
                <img alt="" src={badgeAssets[goal.badgeKey]} />
                <span>
                  <strong>{goal.title}</strong>
                  <i>
                    <b style={{ width: `${Math.max(0, Math.min(100, progress))}%` }} />
                  </i>
                </span>
                <em>{goal.rewardPaid ? "已到账" : goal.isComplete ? "待发放" : `${Math.round(progress)}%`}</em>
              </button>
            );
          })}
        </div>
      ) : null}
    </section>
  );
}
