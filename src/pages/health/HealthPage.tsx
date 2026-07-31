import { useEffect } from "react";
import { formatDuration } from "../../services/formatters";
import { useHealthStore } from "../../stores/healthStore";
import type {
  HealthTrendReport,
  TrendDayPoint,
  TrendRange,
  TrendWeekdayStat,
} from "../../types/health";
import { PixelIcon } from "../../ui/PixelIcon";

const RANGE_OPTIONS: { key: TrendRange; label: string }[] = [
  { key: "days7", label: "7 天" },
  { key: "days30", label: "30 天" },
  { key: "days90", label: "90 天" },
];

const WEEKDAY_LABELS = ["一", "二", "三", "四", "五", "六", "日"];

/** Map a 0-100 health score to a display tone + color token. */
function gradeColor(grade: string): string {
  switch (grade) {
    case "S":
      return "var(--color-brand-orange-strong)";
    case "A":
      return "var(--color-tech-cyan)";
    case "B":
      return "var(--color-status-warning, #f5a623)";
    default:
      return "var(--color-text-muted)";
  }
}

function formatDayLabel(date: string): string {
  // YYYY-MM-DD → MM/DD
  const parts = date.split("-");
  if (parts.length < 3) {
    return date;
  }
  return `${parts[1]}/${parts[2]}`;
}

export function HealthPage() {
  const trendReport = useHealthStore((state) => state.trendReport);
  const range = useHealthStore((state) => state.range);
  const isLoading = useHealthStore((state) => state.isLoading);
  const loadError = useHealthStore((state) => state.loadError);
  const loadHealthTrend = useHealthStore((state) => state.loadHealthTrend);

  useEffect(() => {
    // Load on mount and whenever the range changes.
    void loadHealthTrend(range);
  }, [range, loadHealthTrend]);

  return (
    <div className="cwp-page">
      <div className="page-title-row cwp-health-title-row">
        <h2 className="page-title">健康体检报告</h2>
        <div className="cwp-health-range-tabs">
          {RANGE_OPTIONS.map((opt) => (
            <button
              key={opt.key}
              type="button"
              className={`cwp-health-range-tab${range === opt.key ? " is-active" : ""}`}
              onClick={() => useHealthStore.getState().setRange(opt.key)}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {loadError ? (
        <div className="cwp-health-empty">加载失败：{loadError}</div>
      ) : isLoading || !trendReport ? (
        <div className="cwp-health-empty">正在生成健康趋势…</div>
      ) : (
        <HealthReportBody report={trendReport} />
      )}
    </div>
  );
}

function HealthReportBody({ report }: { report: HealthTrendReport }) {
  const hasData = report.windowDays > 0;

  return (
    <div className="cwp-health-content">
      {/* Hero: health score + grade + summary */}
      <HealthHero report={report} />

      {hasData ? (
        <>
          {/* Score trend bar chart */}
          <ScoreTrendChart series={report.scoreSeries} range={report.range} />

          {/* Streak + delta */}
          <div className="cwp-health-stat-row">
            <StreakCard report={report} />
            <DeltaCard report={report} />
          </div>

          {/* Five-dimension averages */}
          <DimensionAverages report={report} />

          {/* Weekday productivity breakdown */}
          <WeekdayBreakdown stats={report.weekdayBreakdown} />

          {/* Peaks */}
          <PeaksCard report={report} />
        </>
      ) : (
        <div className="cwp-health-empty">
          CoCat 还没有积累到可分析的样本。保持常驻几天后，这里会生成完整的健康趋势画像。
        </div>
      )}
    </div>
  );
}

function HealthHero({ report }: { report: HealthTrendReport }) {
  const color = gradeColor(report.healthGrade);
  return (
    <section className="cwp-health-hero">
      <div className="cwp-health-hero-score" style={{ color }}>
        <span className="cwp-health-hero-grade">{report.healthGrade}</span>
        <span className="cwp-health-hero-num">{report.healthScore}</span>
      </div>
      <div className="cwp-health-hero-info">
        <span className="cwp-health-hero-title">
          <PixelIcon name="shield" size={16} />
          综合健康分
        </span>
        <p className="cwp-health-hero-summary">{report.summary}</p>
      </div>
    </section>
  );
}

function ScoreTrendChart({
  series,
  range,
}: {
  series: TrendDayPoint[];
  range: TrendRange;
}) {
  // For wide windows (30/90), thin the labels so they don't overlap.
  const labelEvery = range === "days7" ? 1 : range === "days30" ? 5 : 15;
  const maxScore = 100;

  return (
    <section className="cwp-health-section">
      <div className="cwp-section-title">
        <PixelIcon name="workScore" size={14} />
        <strong>每日工作分趋势</strong>
        <span className="cwp-health-section-sub">
          {series.filter((p) => p.hasData).length}/{series.length} 天有记录
        </span>
      </div>
      <div
        className="cwp-health-chart"
        style={{ gridTemplateColumns: `repeat(${series.length}, minmax(0, 1fr))` }}
      >
        {series.map((point, index) => (
          <div
            key={point.date}
            className={`cwp-health-bar-col${point.hasData ? "" : " is-empty"}`}
            title={`${formatDayLabel(point.date)}：${point.hasData ? `${point.totalScore} 分` : "无记录"}`}
          >
            <div
              className="cwp-health-bar"
              style={{
                height: point.hasData
                  ? `${Math.max(2, (point.totalScore / maxScore) * 100)}%`
                  : "4%",
              }}
            />
            {index % labelEvery === 0 ? (
              <span className="cwp-health-bar-label">
                {formatDayLabel(point.date)}
              </span>
            ) : null}
          </div>
        ))}
      </div>
    </section>
  );
}

function StreakCard({ report }: { report: HealthTrendReport }) {
  return (
    <section className="cwp-health-stat-card">
      <div className="cwp-section-title">
        <PixelIcon name="calendar" size={14} />
        <strong>连续打卡</strong>
      </div>
      <div className="cwp-health-stat-grid">
        <div className="cwp-health-stat-item">
          <span className="cwp-health-stat-val">{report.streaks.current}</span>
          <span className="cwp-health-stat-lbl">当前连续</span>
        </div>
        <div className="cwp-health-stat-item">
          <span className="cwp-health-stat-val">{report.streaks.longest}</span>
          <span className="cwp-health-stat-lbl">最长连续</span>
        </div>
        <div className="cwp-health-stat-item">
          <span className="cwp-health-stat-val">{report.streaks.totalActiveDays}</span>
          <span className="cwp-health-stat-lbl">活跃天数</span>
        </div>
      </div>
    </section>
  );
}

function DeltaCard({ report }: { report: HealthTrendReport }) {
  const delta = report.deltaVsPrev;
  const toneClass = `is-${delta.tone}`;
  const sign = (v: number) => (v > 0 ? `+${v.toFixed(1)}` : v.toFixed(1));
  return (
    <section className="cwp-health-stat-card">
      <div className="cwp-section-title">
        <PixelIcon name="sparkle" size={14} />
        <strong>环比变化</strong>
      </div>
      <div className="cwp-health-stat-grid">
        <div className={`cwp-health-stat-item ${toneClass}`}>
          <span className="cwp-health-stat-val">{sign(delta.scoreDelta)}</span>
          <span className="cwp-health-stat-lbl">平均分</span>
        </div>
        <div className={`cwp-health-stat-item ${toneClass}`}>
          <span className="cwp-health-stat-val">{sign(delta.hoursDelta)}h</span>
          <span className="cwp-health-stat-lbl">日均时长</span>
        </div>
      </div>
      <p className="cwp-health-stat-note">
        与上一个同等周期对比
      </p>
    </section>
  );
}

const DIMENSIONS: { key: keyof TrendDayPoint; label: string; max: number }[] = [
  { key: "durationScore", label: "运行时长", max: 30 },
  { key: "loadScore", label: "负载强度", max: 25 },
  { key: "complexityScore", label: "任务复杂", max: 20 },
  { key: "stabilityScore", label: "系统稳定", max: 15 },
  { key: "continuityScore", label: "连续投入", max: 10 },
];

function DimensionAverages({ report }: { report: HealthTrendReport }) {
  // Average each dimension across data-days via the series (cheap, client-side).
  const dataPoints = report.scoreSeries.filter((p) => p.hasData);
  const n = dataPoints.length || 1;
  const avgs = DIMENSIONS.map((dim) => {
    const sum = dataPoints.reduce((acc, p) => acc + (p[dim.key] as number), 0);
    return { ...dim, avg: sum / n };
  });

  return (
    <section className="cwp-health-section">
      <div className="cwp-section-title">
        <PixelIcon name="tools" size={14} />
        <strong>五维均值</strong>
        <span className="cwp-health-section-sub">
          日均 {report.averages.activeHours.toFixed(1)}h · CPU {Math.round(report.averages.cpuAvg)}%
        </span>
      </div>
      <div className="cwp-health-dim-list">
        {avgs.map((dim) => (
          <div key={dim.key} className="cwp-health-dim-row">
            <span className="cwp-health-dim-label">{dim.label}</span>
            <div className="cwp-health-dim-bar">
              <span
                className="cwp-health-dim-fill"
                style={{ width: `${Math.min(100, (dim.avg / dim.max) * 100)}%` }}
              />
            </div>
            <span className="cwp-health-dim-val">
              {dim.avg.toFixed(0)}/{dim.max}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}

function WeekdayBreakdown({ stats }: { stats: TrendWeekdayStat[] }) {
  const maxAvg = Math.max(1, ...stats.map((s) => s.avgScore));
  return (
    <section className="cwp-health-section">
      <div className="cwp-section-title">
        <PixelIcon name="calendar" size={14} />
        <strong>工作日画像</strong>
        <span className="cwp-health-section-sub">哪天最高产</span>
      </div>
      <div className="cwp-health-weekday">
        {stats.map((stat) => (
          <div key={stat.weekday} className="cwp-health-weekday-col">
            <div className="cwp-health-weekday-bar-track">
              <span
                className="cwp-health-weekday-bar"
                style={{
                  height: `${Math.max(2, (stat.avgScore / maxAvg) * 100)}%`,
                  opacity: stat.sampleDays === 0 ? 0.2 : 1,
                }}
              />
            </div>
            <span className="cwp-health-weekday-label">{WEEKDAY_LABELS[stat.weekday]}</span>
            <span className="cwp-health-weekday-val">
              {stat.sampleDays > 0 ? Math.round(stat.avgScore) : "-"}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}

function PeaksCard({ report }: { report: HealthTrendReport }) {
  const p = report.peaks;
  return (
    <section className="cwp-health-section">
      <div className="cwp-section-title">
        <PixelIcon name="sparkle" size={14} />
        <strong>周期亮点</strong>
      </div>
      <div className="cwp-health-peaks">
        <PeakItem
          icon="workScore"
          label="最高分"
          date={p.bestScoreDate}
          value={p.bestScore != null ? `${p.bestScore} 分` : "—"}
        />
        <PeakItem
          icon="clock"
          label="最长工作日"
          date={p.longestDayDate}
          value={p.longestHours != null ? formatDuration(p.longestHours * 3600) : "—"}
        />
        <PeakItem
          icon="temp"
          label="最热日"
          date={p.hottestDayDate}
          value={p.hottestThermal != null ? `${Math.round(p.hottestThermal)}% 热压` : "—"}
        />
      </div>
    </section>
  );
}

function PeakItem({
  icon,
  label,
  date,
  value,
}: {
  icon: "workScore" | "clock" | "temp";
  label: string;
  date: string | null;
  value: string;
}) {
  return (
    <div className="cwp-health-peak-item">
      <PixelIcon name={icon} size={16} />
      <div className="cwp-health-peak-info">
        <span className="cwp-health-peak-label">{label}</span>
        <span className="cwp-health-peak-value">{value}</span>
        <span className="cwp-health-peak-date">{date ? formatDayLabel(date) : "—"}</span>
      </div>
    </div>
  );
}
