import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type {
  AssessmentInsight,
  BaselineMetric,
  DailyWorkAssessment,
  DailyWorkAssessmentSummary,
  DailyWorkAssessmentTrend,
  ProcessUsageInsight,
  WorkTimelineSegment,
} from "../../types/dailyWorkAssessment";
import type { WorkLogReport } from "../../types/workLog";
import type { RhythmProfile } from "../../types/rhythm";
import { formatBytes, formatDuration } from "../../services/formatters";
import { copyTextToClipboard } from "../../services/clipboard";
import { trackAchievementEvent } from "../../services/tauriCommands";
import { useWorkLogStore } from "../../stores/workLogStore";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";
import { WorkProfileAvatar } from "../../components/WorkProfileAvatar";
import { TodaySuggestionsStrip } from "./TodaySuggestionsStrip";

function todayKey() {
  return new Date().toISOString().slice(0, 10);
}

function buildRecentDates(count: number) {
  return Array.from({ length: count }, (_, index) => {
    const date = new Date();
    date.setDate(date.getDate() - index);
    return date.toISOString().slice(0, 10);
  });
}

function buildMonthDates(selectedDate: string) {
  const monthKey = selectedDate.slice(0, 7);
  const [year, month] = monthKey.split("-").map(Number);
  if (!year || !month) return [];

  const today = todayKey();
  const todayMonthKey = today.slice(0, 7);
  const monthDays = new Date(year, month, 0).getDate();
  const endDay =
    monthKey === todayMonthKey ? Math.min(monthDays, Number(today.slice(8))) : monthDays;

  return Array.from({ length: endDay }, (_, index) => {
    const day = String(endDay - index).padStart(2, "0");
    return `${monthKey}-${day}`;
  });
}

function formatDateLabel(date: string) {
  const [year, month, day] = date.split("-");
  return `${year}/${month}/${day}`;
}

function formatShortDateLabel(date: string) {
  const [, month, day] = date.split("-");
  return `${month}/${day}`;
}

type WorkLogTab =
  | "overview"
  | "history"
  | "trend"
  | "workprint"
  | "timeline"
  | "insights"
  | "dimensions"
  | "rhythm"
  | "suggestions";

const workLogTabs: Array<{ id: WorkLogTab; label: string }> = [
  { id: "overview", label: "概览" },
  { id: "history", label: "历史" },
  { id: "trend", label: "趋势" },
  { id: "workprint", label: "指纹" },
  { id: "timeline", label: "时间线" },
  { id: "insights", label: "进程" },
  { id: "dimensions", label: "五维" },
  { id: "rhythm", label: "节律" },
  { id: "suggestions", label: "今日建议" },
];

const WORKLOG_HISTORY_LIMIT = 370;
const RECENT_DATE_SHORTCUT_DAYS = 7;
const WORKPRINT_CALENDAR_DAYS = 14;

export function WorkLogPage() {
  const report = useWorkLogStore((state) => state.report);
  const assessment = useWorkLogStore((state) => state.assessment);
  const assessmentHistory = useWorkLogStore((state) => state.assessmentHistory);
  const assessmentTrend = useWorkLogStore((state) => state.assessmentTrend);
  const selectedDate = useWorkLogStore((state) => state.selectedDate);
  const setSelectedDate = useWorkLogStore((state) => state.setSelectedDate);
  const loadAssessmentHistory = useWorkLogStore(
    (state) => state.loadAssessmentHistory,
  );
  const loadAssessmentTrend = useWorkLogStore((state) => state.loadAssessmentTrend);
  const loadRhythmProfile = useWorkLogStore((state) => state.loadRhythmProfile);
  const rhythmProfile = useWorkLogStore((state) => state.rhythmProfile);
  const loadWorkLogReport = useWorkLogStore((state) => state.loadWorkLogReport);
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<WorkLogTab>("overview");
  const [exportHint, setExportHint] = useState<string | null>(null);
  const exportHintTimerRef = useRef<number | null>(null);
  const recentDates = useMemo(() => buildRecentDates(RECENT_DATE_SHORTCUT_DAYS), []);

  useEffect(() => {
    void loadWorkLogReport(selectedDate);
  }, [loadWorkLogReport, selectedDate]);

  useEffect(() => {
    void loadAssessmentHistory(WORKLOG_HISTORY_LIMIT);
    void loadAssessmentTrend(RECENT_DATE_SHORTCUT_DAYS);
    void loadRhythmProfile();
  }, [loadAssessmentHistory, loadAssessmentTrend, loadRhythmProfile]);

  const currentReport = report;
  const currentAssessment = assessment;
  const isToday = selectedDate === todayKey();

  useEffect(
    () => () => {
      if (exportHintTimerRef.current !== null) {
        window.clearTimeout(exportHintTimerRef.current);
      }
    },
    [],
  );

  function selectDate(date: string) {
    setSelectedDate(date);
    setCalendarOpen(false);
  }

  function showExportHint(message: string) {
    if (exportHintTimerRef.current !== null) {
      window.clearTimeout(exportHintTimerRef.current);
    }
    setExportHint(message);
    exportHintTimerRef.current = window.setTimeout(() => {
      setExportHint(null);
      exportHintTimerRef.current = null;
    }, 2200);
  }

  async function handleExportReportCard() {
    const now = Date.now();
    const text = buildReportCardExportText({
      assessment: currentAssessment,
      exportedAt: now,
      report: currentReport,
      selectedDate,
      trend: assessmentTrend,
    });

    try {
      await copyTextToClipboard(text);
      await trackAchievementEvent({
        eventName: "share.report_card.export",
        occurredAt: now,
        idempotencyKey: `share.report_card.export:${selectedDate}:${now}`,
        payload: {
          date: selectedDate,
          dayType: currentAssessment?.dayType ?? "unknown",
          format: "text",
          score: currentAssessment?.score ?? currentReport?.totalScore ?? 0,
        },
        source: "worklog-page",
      });
      showExportHint("日报卡已复制");
    } catch (error) {
      console.error("Failed to export work report card", error);
      showExportHint("复制失败，请重试");
    }
  }

  async function handleExportVisualReportCard() {
    const now = Date.now();

    try {
      await exportVisualReportCard({
        assessment: currentAssessment,
        exportedAt: now,
        report: currentReport,
        selectedDate,
        trend: assessmentTrend,
      });
      await trackAchievementEvent({
        eventName: "share.report_card.export",
        occurredAt: now,
        idempotencyKey: `share.report_card.export.png:${selectedDate}:${now}`,
        payload: {
          date: selectedDate,
          dayType: currentAssessment?.dayType ?? "unknown",
          format: "png",
          score: currentAssessment?.score ?? currentReport?.totalScore ?? 0,
        },
        source: "worklog-page",
      });
      showExportHint("图片卡已导出");
    } catch (error) {
      console.error("Failed to export visual work report card", error);
      showExportHint("图片导出失败");
    }
  }

  return (
    <div className="cwp-page cwp-worklog-page">
      <div className="page-title-row cwp-worklog-title-row">
        <h2 className="page-title">每日工况报告</h2>
        <div className="cwp-worklog-title-actions">
          {exportHint ? (
            <span className="cwp-worklog-export-hint">{exportHint}</span>
          ) : null}
          <button
            className="cwp-worklog-date-btn"
            onClick={() => setCalendarOpen((open) => !open)}
            type="button"
          >
            <PixelIcon name="calendar" size={12} /> {formatDateLabel(selectedDate)}
          </button>
          <button
            className="cwp-worklog-export-btn"
            onClick={() => {
              void handleExportReportCard();
            }}
            type="button"
          >
            复制报告卡
          </button>
          <button
            className="cwp-worklog-export-btn"
            onClick={() => {
              void handleExportVisualReportCard();
            }}
            type="button"
          >
            导出图片卡
          </button>
        </div>
      </div>

      <nav className="cwp-worklog-tabs" aria-label="每日工况报告模块切换">
        {workLogTabs.map((tab) => (
          <button
            className={activeTab === tab.id ? "is-active" : ""}
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            type="button"
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className={`cwp-worklog-tab-panel is-${activeTab}`}>
        {activeTab === "overview" && (
          <>
            <DailyReportHero
              assessment={currentAssessment}
              isToday={isToday}
              report={currentReport}
            />
            <CoreCatCommentaryPanel assessment={currentAssessment} />
          </>
        )}

        {activeTab === "history" && (
          <section className="cwp-history-card-wall">
            <div className="cwp-section-title">
              <PixelIcon name="calendar" size={14} />
              <strong>历史工作画像</strong>
            </div>
            <MonthlyWorkStarMap
              assessment={currentAssessment}
              cards={assessmentHistory}
              onSelect={selectDate}
              selectedDate={selectedDate}
            />
          </section>
        )}

        {activeTab === "trend" && (
          <WorkTrendPanel assessment={currentAssessment} trend={assessmentTrend} />
        )}

        {activeTab === "workprint" && (
          <WorkprintAndBaseline
            assessment={currentAssessment}
            cards={assessmentHistory}
            onSelect={selectDate}
            selectedDate={selectedDate}
          />
        )}

        {activeTab === "timeline" && (
          <section className="cwp-work-timeline-card">
            <MvpSegmentPanel segments={currentAssessment?.mvpSegments ?? []} compact />
            <WorkTimeline segments={currentAssessment?.timeline ?? []} />
          </section>
        )}

        {activeTab === "insights" && (
          <ProcessInsightPanel assessment={currentAssessment} />
        )}

        {activeTab === "dimensions" && (
          <WorkDimensionPanel
            assessment={currentAssessment}
            isToday={isToday}
            report={currentReport}
          />
        )}

        {activeTab === "rhythm" && (
          <RhythmHeatmapPanel profile={rhythmProfile} />
        )}

        {activeTab === "suggestions" && (
          <TodaySuggestionsStrip />
        )}
      </div>

      {calendarOpen && (
        <div className="cwp-worklog-calendar-popover">
          <div className="cwp-worklog-calendar-head">
            <span>选择日报日期</span>
            <button onClick={() => setCalendarOpen(false)} type="button">
              ×
            </button>
          </div>
          <input
            className="cwp-worklog-date-input"
            max={todayKey()}
            onChange={(event) => selectDate(event.target.value)}
            type="date"
            value={selectedDate}
          />
          <div className="cwp-worklog-date-grid">
            {recentDates.map((date) => (
              <button
                className={date === selectedDate ? "is-active" : ""}
                key={date}
                onClick={() => selectDate(date)}
                type="button"
              >
                {formatDateLabel(date)}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function RarityMetricsPanel({ reason }: { reason?: string }) {
  const metrics = useMemo(() => {
    if (!reason) return null;
    const regex = /(?:预览)?画像分\s*(\d+)[，,]\s*连续活跃\s*(\d+)\s*天[，,]\s*负载指数\s*([\d.]+)[，,]\s*稳定指数\s*([\d.]+)/;
    const match = reason.match(regex);
    if (match) {
      return {
        score: parseInt(match[1], 10),
        streak: parseInt(match[2], 10),
        load: Math.round(parseFloat(match[3])),
        stability: Math.round(parseFloat(match[4])),
      };
    }
    return null;
  }, [reason]);

  if (!metrics) {
    if (!reason || reason === "等待更多数据" || reason.includes("等待更多数据")) {
      return null;
    }
    return (
      <div className="cwp-daily-rarity-reason">
        <PixelIcon name="info" size={10} />
        <span>{reason}</span>
      </div>
    );
  }

  return (
    <div className="cwp-rarity-metrics-row">
      <div className="cwp-rarity-metric-card" title="当前工况下的工作画像综合评分">
        <span className="metric-icon">
          <PixelIcon name="workScore" size={14} style={{ color: "var(--color-insight-gold)" }} />
        </span>
        <div className="metric-content">
          <span className="metric-label">画像评分</span>
          <strong className="metric-value color-gold">{metrics.score}</strong>
        </div>
      </div>
      <div className="cwp-rarity-metric-card" title="连续在工况下保持活跃的天数">
        <span className="metric-icon">
          <PixelIcon name="calendar" size={14} style={{ color: "var(--color-brand-orange-strong)" }} />
        </span>
        <div className="metric-content">
          <span className="metric-label">连续活跃</span>
          <strong className="metric-value color-orange">
            {metrics.streak}<small>天</small>
          </strong>
        </div>
      </div>
      <div className="cwp-rarity-metric-card" title="系统综合负载与压力指标">
        <span className="metric-icon">
          <PixelIcon name="energy" size={14} style={{ color: "var(--color-success)" }} />
        </span>
        <div className="metric-content">
          <span className="metric-label">负载指数</span>
          <strong className="metric-value color-green">{metrics.load}</strong>
        </div>
      </div>
      <div className="cwp-rarity-metric-card" title="操作与输入节奏的稳定性指数">
        <span className="metric-icon">
          <PixelIcon name="shield" size={14} style={{ color: "var(--color-tech-cyan)" }} />
        </span>
        <div className="metric-content">
          <span className="metric-label">稳定指数</span>
          <strong className="metric-value color-cyan">{metrics.stability}</strong>
        </div>
      </div>
    </div>
  );
}

function DailyReportHero({
  assessment,
  isToday,
  report,
}: {
  assessment: DailyWorkAssessment | null;
  isToday: boolean;
  report: WorkLogReport | null;
}) {
  return (
    <section className={`cwp-daily-report-hero is-rarity-${rarityTierClass(assessment?.rarity.tier)}`}>
      <div className="cwp-daily-report-copy">
        <span className="cwp-daily-report-eyebrow">
          {isToday ? "Today Report" : "History Report"}
        </span>
        <div className="cwp-daily-report-title-row">
          <h3>{assessment?.dayTypeTitle ?? "数据积累中"}</h3>
          <span className={`cwp-rarity-chip is-${rarityTierClass(assessment?.rarity.tier)}`}>
            {assessment?.rarity.label ?? "C 级工况卡"}
          </span>
          <span className="cwp-daily-title-chip">
            {assessment?.title.title ?? "观察记录员"} Lv.{assessment?.title.level ?? 1}
          </span>
        </div>
        <p>
          {assessment?.corecatSummary ??
            report?.summary ??
            "保持 CoreCat 运行后，将自动生成当天工作画像。"}
        </p>
        <RarityMetricsPanel reason={assessment?.rarity.reason} />
        <div className="cwp-daily-badge-row">
          {(assessment?.badgeIds ?? ["OBSERVE"]).map((badge) => (
            <span key={badge}>{badge}</span>
          ))}
        </div>
      </div>
      <div className={`cwp-daily-score-card is-rarity-${rarityTierClass(assessment?.rarity.tier)}`}>
        <WorkProfileAvatar
          dayType={assessment?.dayType}
        />
      </div>
    </section>
  );
}

function CoreCatCommentaryPanel({
  assessment,
}: {
  assessment: DailyWorkAssessment | null;
}) {
  return (
    <section
      className={`cwp-corecat-commentary is-${assessment?.corecatCommentary.tone ?? "tease"
        }`}
    >
      <div className="cwp-section-title">
        <PixelIcon name="cat" size={14} />
        <strong>{assessment?.corecatCommentary.title ?? "CoreCat 正在观察"}</strong>
      </div>
      <p>
        {assessment?.corecatCommentary.body ??
          "CoreCat 会在采样充足后给出更有性格的日报点评。"}
      </p>
    </section>
  );
}

function createEmptyWorkprint(): DailyWorkAssessmentSummary["workprint"] {
  return {
    label: "数据积累中",
    description: "该日期暂无足够采样，CoreCat 还没有形成稳定工作指纹。",
    pixelGrid: [],
    width: 8,
    height: 8,
    loadShape: 0,
    inputRhythm: 0,
    ioIntensity: 0,
    thermalPressure: 0,
    continuity: 0,
  };
}

function createEmptyAssessmentSummary(date: string): DailyWorkAssessmentSummary {
  return {
    date,
    dayType: "unknown",
    dayTypeTitle: "空日期",
    rarity: {
      tier: "C",
      label: "未形成",
      score: 0,
      reason: "该日期暂无足够采样",
    },
    title: {
      family: "empty",
      title: "暂无画像",
      level: 1,
      progress: 0,
      nextLevelAt: null,
    },
    workprint: createEmptyWorkprint(),
    score: 0,
    corecatSummary: "该日期暂无足够采样。",
    badgeIds: ["EMPTY"],
    hasTimeline: false,
    hasData: false,
  };
}

function assessmentToSummary(assessment: DailyWorkAssessment): DailyWorkAssessmentSummary {
  const hasData =
    assessment.score > 0 ||
    assessment.dayType !== "unknown" ||
    assessment.timeline.length > 0 ||
    assessment.badgeIds.some((badge) => badge !== "EMPTY");

  return {
    date: assessment.date,
    dayType: assessment.dayType,
    dayTypeTitle: assessment.dayTypeTitle,
    rarity: assessment.rarity,
    title: assessment.title,
    workprint: assessment.workprint,
    score: assessment.score,
    corecatSummary: assessment.corecatSummary,
    badgeIds: assessment.badgeIds,
    hasTimeline: assessment.timeline.length > 0,
    hasData,
  };
}

function buildAssessmentSummaryMap(
  cards: DailyWorkAssessmentSummary[],
  assessment?: DailyWorkAssessment | null,
) {
  const map = new Map<string, DailyWorkAssessmentSummary>();
  cards.forEach((card) => map.set(card.date, card));
  if (assessment) {
    map.set(assessment.date, assessmentToSummary(assessment));
  }
  return map;
}

function WorkprintAndBaseline({
  assessment,
  cards,
  onSelect,
  selectedDate,
}: {
  assessment: DailyWorkAssessment | null;
  cards: DailyWorkAssessmentSummary[];
  onSelect: (date: string) => void;
  selectedDate: string;
}) {
  const baselineMetrics = assessment?.baseline.metrics ?? [];

  return (
    <>
      <section className="cwp-daily-report-grid">
        <article className="cwp-workprint-card">
          <div className="cwp-section-title">
            <PixelIcon name="puzzle" size={14} />
            <strong>Workprint 工作指纹</strong>
          </div>
          <div className="cwp-workprint-body">
            <WorkprintGrid
              grid={assessment?.workprint.pixelGrid ?? []}
              height={assessment?.workprint.height ?? 8}
              width={assessment?.workprint.width ?? 8}
            />
            <div className="cwp-workprint-copy">
              <strong>{assessment?.workprint.label ?? "数据积累中"}</strong>
              <p>
                {assessment?.workprint.description ??
                  "CoreCat 正在等待更多本地采样，稍后会生成今天独有的数据纹理。"}
              </p>
            </div>
          </div>
        </article>

        <article className="cwp-baseline-card">
          <div className="cwp-section-title">
            <PixelIcon name="clock" size={14} />
            <strong>个人基线对比</strong>
          </div>
          <p>
            {assessment?.baseline.summary ??
              "连续使用几天后，CoreCat 会用近 7 日记录对比今天的节奏。"}
          </p>
          <div className="cwp-baseline-pill-grid">
            {baselineMetrics.map((metric) => (
              <BaselinePill key={metric.key} metric={metric} />
            ))}
          </div>
        </article>
      </section>
      <RecentWorkprintCalendar
        assessment={assessment}
        cards={cards}
        onSelect={onSelect}
        selectedDate={selectedDate}
      />
    </>
  );
}

function formatWorkprintTotalScore(score: number, hasData: boolean) {
  if (!hasData) return "--";
  return String(Math.round(Math.max(0, score)));
}

function RecentWorkprintCalendar({
  assessment,
  cards,
  onSelect,
  selectedDate,
}: {
  assessment: DailyWorkAssessment | null;
  cards: DailyWorkAssessmentSummary[];
  onSelect: (date: string) => void;
  selectedDate: string;
}) {
  const summaryMap = buildAssessmentSummaryMap(cards, assessment);
  const calendarCards = buildRecentDates(WORKPRINT_CALENDAR_DAYS).map(
    (date) => summaryMap.get(date) ?? createEmptyAssessmentSummary(date),
  );

  return (
    <section className="cwp-workprint-calendar-card">
      <div className="cwp-section-title">
        <PixelIcon name="calendar" size={14} />
        <strong>最近 {WORKPRINT_CALENDAR_DAYS} 天指纹日历</strong>
      </div>
      <div className="cwp-workprint-calendar-grid">
        {calendarCards.map((card) => (
          <button
            className={`cwp-workprint-day ${card.hasData ? "" : "is-empty"} ${card.date === selectedDate ? "is-active" : ""
              }`}
            key={card.date}
            onClick={() => onSelect(card.date)}
            title={`${formatDateLabel(card.date)} ${card.workprint.label}`}
            type="button"
          >
            <span className="cwp-workprint-day-date">{formatShortDateLabel(card.date)}</span>
            <WorkprintGrid
              grid={card.workprint.pixelGrid}
              height={card.workprint.height}
              width={card.workprint.width}
            />
            <div className="cwp-workprint-day-score">
              <em>总分</em>
              <strong>{formatWorkprintTotalScore(card.score, card.hasData)}</strong>
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}

function MvpSegmentPanel({
  compact = false,
  segments,
}: {
  compact?: boolean;
  segments: WorkTimelineSegment[];
}) {
  if (segments.length === 0) {
    return compact ? null : (
      <section className="cwp-mvp-segments is-empty">
        <div className="cwp-mvp-label">
          <PixelIcon name="energy" size={10} />
          <span>今日 MVP 片段</span>
        </div>
        <p>CoreCat 还没有抓到足够清晰的高光片段。</p>
      </section>
    );
  }

  return (
    <section className={`cwp-mvp-segments${compact ? " is-compact" : ""}`}>
      <div className="cwp-mvp-label">
        <PixelIcon name="energy" size={10} />
        <span>今日 MVP 片段</span>
      </div>
      <div className="cwp-mvp-segment-grid">
        {segments.map((segment, index) => (
          <article
            className={`cwp-mvp-segment is-${segment.kind}`}
            key={`${segment.startTime}-${segment.endTime}-${segment.kind}`}
          >
            <span>#{index + 1}</span>
            <div>
              <strong>{segment.label}</strong>
              <em>
                {segment.startTime} - {segment.endTime} · 强度{" "}
                {Math.round(segment.intensity)}
              </em>
            </div>
            <p>{segment.description}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function MonthlyWorkStarMap({
  assessment,
  cards,
  onSelect,
  selectedDate,
}: {
  assessment: DailyWorkAssessment | null;
  cards: DailyWorkAssessmentSummary[];
  onSelect: (date: string) => void;
  selectedDate: string;
}) {
  const selectedMonth = selectedDate.slice(0, 7);
  const monthDates = buildMonthDates(selectedDate);
  const summaryMap = buildAssessmentSummaryMap(
    cards.filter((card) => card.date.startsWith(selectedMonth)),
    assessment?.date.startsWith(selectedMonth) ? assessment : null,
  );
  const monthCards = monthDates.map(
    (date) => summaryMap.get(date) ?? createEmptyAssessmentSummary(date),
  );

  if (monthCards.length === 0) {
    return (
      <div className="cwp-history-card-empty">
        这个月份暂时没有可回看的历史画像。保持常驻后，这里会按日期生成工作卡片墙。
      </div>
    );
  }

  return (
    <div className="cwp-work-star-map">
      <section className="cwp-work-star-month">
        <div className="cwp-work-star-month-head">
          <strong>{selectedMonth.replace("-", "/")}</strong>
          <span>{monthCards.filter((card) => card.hasData).length} 张工况卡</span>
        </div>
        <div className="cwp-work-star-grid">
          {monthCards.map((card) => (
            <button
              className={`cwp-work-star is-${rarityTierClass(card.rarity.tier)} ${card.hasData ? "" : "is-empty"
                } ${card.date === selectedDate ? "is-active" : ""}`}
              key={card.date}
              onClick={() => onSelect(card.date)}
              title={`${formatDateLabel(card.date)} ${card.dayTypeTitle} ${card.rarity.label}`}
              type="button"
            >
              <div className="cwp-work-star-head">
                <span className="cwp-work-star-day">{card.date.slice(8)}</span>
                <strong>{card.hasData ? card.score : "--"}</strong>
              </div>
              <WorkProfileAvatar
                className="cwp-work-star-avatar"
                dayType={card.dayType}
              />
              <span className={`cwp-work-star-rarity is-${rarityTierClass(card.rarity.tier)}`}>
                {card.hasData ? card.rarity.label : "未形成"}
              </span>
              <div className="cwp-work-star-copy">
                <strong>{card.hasData ? card.dayTypeTitle : "空日期"}</strong>
                <em>{card.hasData ? `${card.title.title} Lv.${card.title.level}` : "暂无画像"}</em>
              </div>
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

function AssessmentInsightGrid({
  assessment,
}: {
  assessment: DailyWorkAssessment | null;
}) {
  return (
    <section className="cwp-assessment-insights">
      <InsightColumn
        icon="energy"
        items={assessment?.highlights ?? []}
        title="今日亮点"
      />
      <InsightColumn
        icon="shield"
        items={assessment?.risks ?? []}
        title="今日隐患"
      />
      <InsightColumn
        icon="settings"
        items={assessment?.suggestions ?? []}
        title="明日建议"
      />
    </section>
  );
}

function WorkDimensionPanel({
  assessment,
  isToday,
  report,
}: {
  assessment: DailyWorkAssessment | null;
  isToday: boolean;
  report: WorkLogReport | null;
}) {
  const dimensions = assessment?.dimensions ?? report?.dimensions ?? [];

  return (
    <section className="cwp-worklog-layout cwp-worklog-layout-assessment">
      <div className="cwp-worklog-score-panel">
        <div className="cwp-worklog-score-ring">
          <strong>{assessment?.score ?? report?.totalScore ?? 0}</strong>
          <span>/ 100</span>
        </div>
        <div className="cwp-worklog-score-copy">
          <span>{isToday ? "今日五维画像" : "历史五维画像"}</span>
          <p>{report?.summary ?? "暂无足够数据。"}</p>
        </div>
        <div className="cwp-worklog-mini-stats">
          <div>
            <span>运行时长</span>
            <strong>{formatDuration(report?.activeSeconds ?? 0)}</strong>
          </div>
          <div>
            <span>采样记录</span>
            <strong>{report?.sampleCount ?? 0}</strong>
          </div>
        </div>
      </div>

      <div className="cwp-worklog-dimension-grid">
        {dimensions.map((dimension) => (
          <article className="cwp-worklog-dimension-card" key={dimension.key}>
            <div className="cwp-worklog-dimension-head">
              <strong>{dimension.title}</strong>
              <span>
                {dimension.score}/{dimension.maxScore}
              </span>
            </div>
            <div className="cwp-worklog-meter">
              <span
                style={{
                  width: `${Math.min(
                    100,
                    (dimension.score / Math.max(1, dimension.maxScore)) * 100,
                  )}%`,
                }}
              />
            </div>
            <em>{dimension.value}</em>
            {(dimension.facts ?? []).length > 0 && (
              <dl className="cwp-worklog-fact-grid">
                {(dimension.facts ?? []).map((fact) => (
                  <div key={`${dimension.key}-${fact.label}`}>
                    <dt>{fact.label}</dt>
                    <dd>{fact.value}</dd>
                  </div>
                ))}
              </dl>
            )}
            <p>{dimension.explanation}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function WorkTrendPanel({
  assessment,
  trend,
}: {
  assessment: DailyWorkAssessment | null;
  trend: DailyWorkAssessmentTrend | null;
}) {
  if (!trend || trend.sampleDays === 0) {
    return (
      <div className="cwp-trend-page">
        <section className="cwp-trend-card">
          <div className="cwp-section-title">
            <PixelIcon name="energy" size={14} />
            <strong>近 14 日趋势洞察</strong>
          </div>
          <div className="cwp-trend-empty">
            CoreCat 还没有足够的历史日报。连续使用几天后，这里会显示近期主导形态、最高画像日和节奏线覆盖。
          </div>
        </section>
        <AssessmentInsightGrid assessment={assessment} />
      </div>
    );
  }

  return (
    <div className="cwp-trend-page">
      <section className={`cwp-trend-card is-${trend.dominantDayType}`}>
        <div className="cwp-section-title">
          <PixelIcon name="energy" size={14} />
          <strong>近 14 日趋势洞察</strong>
        </div>
        <div className="cwp-trend-head">
          <div className="cwp-trend-copy">
            <span>CoreCat Trend</span>
            <p>{trend.summary}</p>
          </div>
          <div className="cwp-trend-stat-grid">
            <TrendStat label="样本" value={`${trend.sampleDays} 天`} />
            <TrendStat label="平均" value={`${trend.averageScore}`} />
            <TrendStat
              label="最高"
              subValue={trend.bestDate ? formatDateLabel(trend.bestDate) : "--"}
              value={trend.bestScore !== null ? `${trend.bestScore}` : "--"}
            />
            <TrendStat label="首尾" value={formatScoreDelta(trend.scoreDelta)} />
          </div>
        </div>
        <div className="cwp-trend-insight-grid">
          {trend.insights.slice(0, 3).map((item) => (
            <article
              className={`cwp-trend-insight is-${item.severity}`}
              key={`${item.title}-${item.metricValue ?? ""}`}
            >
              <div>
                <strong>{item.title}</strong>
                {item.metricValue ? <span>{item.metricValue}</span> : null}
              </div>
              <p>{item.body}</p>
            </article>
          ))}
        </div>
      </section>
      <AssessmentInsightGrid assessment={assessment} />
    </div>
  );
}

function TrendStat({
  label,
  subValue,
  value,
}: {
  label: string;
  subValue?: string;
  value: string;
}) {
  return (
    <div className="cwp-trend-stat">
      <span>{label}</span>
      <strong>{value}</strong>
      {subValue ? <em>{subValue}</em> : null}
    </div>
  );
}

function WorkTimeline({ segments }: { segments: WorkTimelineSegment[] }) {
  if (segments.length === 0) {
    return (
      <div className="cwp-work-timeline-empty">
        CoreCat 正在积累 15 分钟节奏片段。旧日报没有时间片数据时，这里会保持空状态。
      </div>
    );
  }

  return (
    <div className="cwp-work-timeline">
      <div className="cwp-work-timeline-track">
        {segments.map((segment) => (
          <button
            className={`cwp-work-timeline-segment is-${segment.kind}`}
            key={`${segment.startTime}-${segment.endTime}-${segment.kind}`}
            style={
              {
                "--segment-flex": Math.max(1, segment.intensity * 10),
              } as CSSProperties
            }
            title={`${segment.startTime}-${segment.endTime} ${segment.description}`}
            type="button"
          >
            <span>{segment.label}</span>
          </button>
        ))}
      </div>
      <div className="cwp-work-timeline-list">
        {segments.slice(0, 6).map((segment) => (
          <article
            className={`cwp-work-timeline-detail is-${segment.kind}`}
            key={`${segment.startTime}-${segment.endTime}-${segment.label}`}
          >
            <div>
              <strong>{segment.label}</strong>
              <span>
                {segment.startTime} - {segment.endTime}
              </span>
            </div>
            <p>{segment.description}</p>
          </article>
        ))}
      </div>
    </div>
  );
}

function WorkprintGrid({
  grid,
  height,
  width,
}: {
  grid: number[];
  height: number;
  width: number;
}) {
  const cells = grid.length > 0 ? grid : Array.from({ length: width * height }, () => 0);

  return (
    <div
      className="cwp-workprint-grid"
      style={
        {
          "--workprint-cols": width,
        } as CSSProperties
      }
    >
      {cells.map((value, index) => (
        <span
          className={`tone-${Math.max(0, Math.min(4, Math.floor(value)))}`}
          key={`${index}-${value}`}
        />
      ))}
    </div>
  );
}

function BaselinePill({ metric }: { metric: BaselineMetric }) {
  return (
    <div className={`cwp-baseline-pill is-${metric.tone}`}>
      <span>{metric.label}</span>
      <strong>{metric.currentValue}</strong>
      <em>基线 {metric.baselineValue}</em>
      <small>{formatDelta(metric.deltaRatio)}</small>
    </div>
  );
}

function ProcessInsightPanel({
  assessment,
}: {
  assessment: DailyWorkAssessment | null;
}) {
  const processes = assessment?.processInsights ?? [];

  if (processes.length === 0) {
    return (
      <section className="cwp-process-panel">
        <div className="cwp-section-title">
          <PixelIcon name="processPortrait" size={14} />
          <strong>后台进程画像</strong>
        </div>
        <div className="cwp-process-empty">
          CoreCat 还没有积累到可分析的进程采样。保持 Tauri 版本运行一段时间后，这里会展示驻留时间、活跃频率和资源占用最高的后台进程。
        </div>
      </section>
    );
  }

  const topProcess = processes[0];

  return (
    <section className="cwp-process-panel">
      <div className="cwp-process-head">
        <div className="cwp-section-title">
          <PixelIcon name="processPortrait" size={14} />
          <strong>后台进程画像</strong>
        </div>
        <div className="cwp-process-summary">
          <span>最高贡献</span>
          <strong>{topProcess.name}</strong>
          <em>{formatDuration(topProcess.activeSeconds)} 活跃</em>
        </div>
      </div>

      <div className="cwp-process-grid">
        {processes.map((process, index) => (
          <ProcessInsightCard
            key={`${process.name}-${index}`}
            process={process}
            rank={index + 1}
          />
        ))}
      </div>
    </section>
  );
}

function ProcessInsightCard({
  process,
  rank,
}: {
  process: ProcessUsageInsight;
  rank: number;
}) {
  const diskTotal = process.diskReadBytesTotal + process.diskWriteBytesTotal;
  const activeRatio =
    process.observedSeconds > 0
      ? Math.round((process.activeSeconds / process.observedSeconds) * 100)
      : 0;

  return (
    <article className={`cwp-process-card is-${process.severity}`}>
      <div className="cwp-process-card-head">
        <span>#{rank}</span>
        <strong title={process.name}>{process.name}</strong>
        <em>{process.rankLabel}</em>
      </div>
      <p>{process.summary}</p>
      <dl className="cwp-process-metrics">
        <div>
          <dt>驻留</dt>
          <dd>{formatDuration(process.observedSeconds)}</dd>
        </div>
        <div>
          <dt>活跃</dt>
          <dd>{formatDuration(process.activeSeconds)}</dd>
        </div>
        <div>
          <dt>频率</dt>
          <dd>{process.activeSampleCount}/{process.sampleCount}</dd>
        </div>
        <div>
          <dt>CPU</dt>
          <dd>{Math.round(process.cpuPressurePercent)}%</dd>
        </div>
        <div>
          <dt>内存峰值</dt>
          <dd>{formatBytes(process.memoryBytesPeak)}</dd>
        </div>
        <div>
          <dt>磁盘</dt>
          <dd>{formatBytes(diskTotal)}</dd>
        </div>
      </dl>
      <div className="cwp-process-bar">
        <span style={{ width: `${Math.min(100, activeRatio)}%` }} />
      </div>
    </article>
  );
}

function InsightColumn({
  icon,
  items,
  title,
}: {
  icon: PixelIconName;
  items: AssessmentInsight[];
  title: string;
}) {
  const safeItems =
    items.length > 0
      ? items
      : [
        {
          title: "等待更多数据",
          body: "CoreCat 正在积累今天的本地采样。",
          severity: "neutral" as const,
          metricValue: null,
        },
      ];

  return (
    <article className="cwp-assessment-column">
      <div className="cwp-section-title">
        <PixelIcon name={icon} size={14} />
        <strong>{title}</strong>
      </div>
      {safeItems.map((item) => (
        <div
          className={`cwp-assessment-item is-${item.severity}`}
          key={`${title}-${item.title}`}
        >
          <div>
            <strong>{item.title}</strong>
            {item.metricValue ? <span>{item.metricValue}</span> : null}
          </div>
          <p>{item.body}</p>
        </div>
      ))}
    </article>
  );
}

function formatDelta(value: number) {
  const percent = Math.round(Math.abs(value) * 100);
  if (percent === 0) {
    return "±0%";
  }

  return `${value > 0 ? "+" : "-"}${percent}%`;
}

function formatScoreDelta(value: number) {
  if (value === 0) {
    return "±0";
  }

  return `${value > 0 ? "+" : "-"}${Math.abs(value)}`;
}

function rarityTierClass(tier?: string) {
  return (tier ?? "C").toLowerCase();
}

function buildReportCardExportText({
  assessment,
  exportedAt,
  report,
  selectedDate,
  trend,
}: {
  assessment: DailyWorkAssessment | null;
  exportedAt: number;
  report: WorkLogReport | null;
  selectedDate: string;
  trend: DailyWorkAssessmentTrend | null;
}) {
  const dimensions = assessment?.dimensions ?? report?.dimensions ?? [];
  const insightLines = [
    ...formatInsightLines("亮点", assessment?.highlights ?? []),
    ...formatInsightLines("隐患", assessment?.risks ?? []),
    ...formatInsightLines("建议", assessment?.suggestions ?? []),
  ];
  const processLines = formatProcessLines(assessment?.processInsights ?? []);
  const dimensionLines = dimensions.map(
    (dimension) =>
      `- ${dimension.title}: ${dimension.score}/${dimension.maxScore} (${dimension.value})`,
  );

  return [
    "CoworkPal 每日工况报告卡",
    `日期：${formatDateLabel(selectedDate)}`,
    `导出时间：${new Date(exportedAt).toLocaleString("zh-CN")}`,
    `画像：${assessment?.dayTypeTitle ?? "数据积累中"}`,
    `稀有度：${assessment?.rarity.label ?? "C 级工况卡"} (${assessment?.rarity.score ?? 0})`,
    `今日称号：${assessment?.title.title ?? "观察记录员"} Lv.${assessment?.title.level ?? 1}`,
    `分数：${assessment?.score ?? report?.totalScore ?? 0}/100`,
    `运行时长：${formatDuration(report?.activeSeconds ?? 0)}`,
    `采样记录：${report?.sampleCount ?? 0}`,
    `徽章标签：${(assessment?.badgeIds ?? ["OBSERVE"]).join(" / ")}`,
    "",
    "CoreCat 摘要",
    assessment?.corecatSummary ??
    report?.summary ??
    "保持 CoreCat 运行后，将自动生成当天工作画像。",
    assessment?.corecatCommentary
      ? `${assessment.corecatCommentary.title}：${assessment.corecatCommentary.body}`
      : "",
    "",
    "Workprint",
    `${assessment?.workprint.label ?? "数据积累中"}：${assessment?.workprint.description ?? "暂无足够采样。"
    }`,
    "",
    "洞察",
    ...(insightLines.length > 0 ? insightLines : ["- 暂无洞察"]),
    "",
    "后台进程画像",
    ...(processLines.length > 0 ? processLines : ["- 暂无进程画像"]),
    "",
    "MVP 片段",
    ...(assessment?.mvpSegments.length
      ? assessment.mvpSegments.map(
        (segment, index) =>
          `- #${index + 1} ${segment.startTime}-${segment.endTime} ${segment.label}: ${segment.description}`,
      )
      : ["- 暂无高光片段"]),
    "",
    "五维得分",
    ...(dimensionLines.length > 0 ? dimensionLines : ["- 暂无五维数据"]),
    "",
    "近 14 日趋势",
    trend?.summary ?? "暂无足够历史日报。",
  ].join("\n");
}

function formatInsightLines(prefix: string, items: AssessmentInsight[]) {
  return items.map((item) => {
    const metric = item.metricValue ? ` ${item.metricValue}` : "";
    return `- ${prefix} / ${item.title}${metric}: ${item.body}`;
  });
}

function formatProcessLines(items: ProcessUsageInsight[]) {
  return items.map((item, index) => {
    const diskTotal = item.diskReadBytesTotal + item.diskWriteBytesTotal;
    return `- #${index + 1} ${item.name} / ${item.rankLabel}: 驻留 ${formatDuration(
      item.observedSeconds,
    )}，活跃 ${formatDuration(item.activeSeconds)}，活跃采样 ${item.activeSampleCount}/${item.sampleCount}，CPU ${Math.round(
      item.cpuPressurePercent,
    )}%，内存峰值 ${formatBytes(item.memoryBytesPeak)}，磁盘 ${formatBytes(diskTotal)}`;
  });
}

async function exportVisualReportCard({
  assessment,
  exportedAt,
  report,
  selectedDate,
  trend,
}: {
  assessment: DailyWorkAssessment | null;
  exportedAt: number;
  report: WorkLogReport | null;
  selectedDate: string;
  trend: DailyWorkAssessmentTrend | null;
}) {
  const canvas = document.createElement("canvas");
  canvas.width = 960;
  canvas.height = 540;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("canvas context is not available");
  }

  const rarity = assessment?.rarity ?? {
    tier: "C",
    label: "C 级工况卡",
    score: 0,
    reason: "等待更多数据",
  };
  const title = assessment?.title.title ?? "观察记录员";
  const score = assessment?.score ?? report?.totalScore ?? 0;
  const dayTypeTitle = assessment?.dayTypeTitle ?? "数据积累中";

  context.fillStyle = "#0a111d";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.strokeStyle = rarityColor(rarity.tier);
  context.lineWidth = 6;
  context.strokeRect(18, 18, canvas.width - 36, canvas.height - 36);

  context.fillStyle = "rgba(255, 184, 77, 0.08)";
  context.fillRect(34, 34, canvas.width - 68, 128);
  context.fillStyle = "#ffd566";
  context.font = "700 26px sans-serif";
  context.fillText("CoworkPal 每日工况卡", 54, 76);
  context.font = "16px sans-serif";
  context.fillStyle = "#9eb4ca";
  context.fillText(`${formatDateLabel(selectedDate)} · ${dayTypeTitle}`, 54, 106);
  context.fillText(`导出 ${new Date(exportedAt).toLocaleString("zh-CN")}`, 54, 134);

  context.fillStyle = rarityColor(rarity.tier);
  context.font = "700 58px sans-serif";
  context.fillText(rarity.tier, 770, 92);
  context.font = "700 24px sans-serif";
  context.fillText(`${score}/100`, 770, 128);
  context.font = "14px sans-serif";
  context.fillStyle = "#d6e9f4";
  context.fillText(rarity.label, 770, 152);

  drawWorkprintOnCanvas(context, assessment?.workprint, 54, 194, 148);

  context.fillStyle = "#ffffff";
  context.font = "700 28px sans-serif";
  context.fillText(`${title} Lv.${assessment?.title.level ?? 1}`, 228, 214);
  context.fillStyle = "#9eb4ca";
  context.font = "15px sans-serif";
  wrapCanvasText(
    context,
    assessment?.corecatCommentary.body ??
    assessment?.corecatSummary ??
    report?.summary ??
    "保持 CoreCat 运行后，将自动生成当天工作画像。",
    228,
    246,
    650,
    23,
    4,
  );

  context.fillStyle = "#233044";
  context.fillRect(54, 372, 852, 1);
  context.fillStyle = "#ffd566";
  context.font = "700 18px sans-serif";
  context.fillText("MVP 片段", 54, 408);
  context.fillText("近 14 日趋势", 540, 408);

  context.fillStyle = "#d6e9f4";
  context.font = "14px sans-serif";
  const mvpSegments = assessment?.mvpSegments ?? [];
  if (mvpSegments.length > 0) {
    mvpSegments.slice(0, 3).forEach((segment, index) => {
      wrapCanvasText(
        context,
        `#${index + 1} ${segment.startTime}-${segment.endTime} ${segment.label}`,
        54,
        436 + index * 28,
        430,
        18,
        1,
      );
    });
  } else {
    context.fillText("暂无高光片段", 54, 436);
  }
  wrapCanvasText(context, trend?.summary ?? "暂无足够历史日报。", 540, 436, 340, 20, 3);

  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((value) => {
      if (value) {
        resolve(value);
      } else {
        reject(new Error("failed to create report card image"));
      }
    }, "image/png");
  });

  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `coworkpal-report-card-${selectedDate}.png`;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
}

function drawWorkprintOnCanvas(
  context: CanvasRenderingContext2D,
  workprint: DailyWorkAssessment["workprint"] | undefined,
  x: number,
  y: number,
  size: number,
) {
  const width = workprint?.width ?? 8;
  const height = workprint?.height ?? 8;
  const cells = workprint?.pixelGrid.length
    ? workprint.pixelGrid
    : Array.from({ length: width * height }, () => 0);
  const cellSize = Math.floor(size / Math.max(width, height));
  const colors = ["#111827", "#284258", "#3f6f89", "#ffb84d", "#ffd566"];

  context.fillStyle = "#080d16";
  context.fillRect(x - 6, y - 6, cellSize * width + 12, cellSize * height + 12);
  for (let row = 0; row < height; row += 1) {
    for (let column = 0; column < width; column += 1) {
      const value = Math.max(0, Math.min(4, Math.floor(cells[row * width + column] ?? 0)));
      context.fillStyle = colors[value];
      context.fillRect(x + column * cellSize, y + row * cellSize, cellSize - 2, cellSize - 2);
    }
  }
}

function wrapCanvasText(
  context: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxWidth: number,
  lineHeight: number,
  maxLines: number,
) {
  const chars = Array.from(text);
  let line = "";
  let lineCount = 0;

  for (const char of chars) {
    const nextLine = `${line}${char}`;
    if (context.measureText(nextLine).width > maxWidth && line) {
      context.fillText(line, x, y + lineCount * lineHeight);
      line = char;
      lineCount += 1;
      if (lineCount >= maxLines) {
        return;
      }
    } else {
      line = nextLine;
    }
  }

  if (line && lineCount < maxLines) {
    context.fillText(line, x, y + lineCount * lineHeight);
  }
}

function rarityColor(tier?: string) {
  switch (tier) {
    case "SS":
      return "#ff7ab6";
    case "S":
      return "#ffd566";
    case "A":
      return "#7dd3fc";
    case "B":
      return "#5cda71";
    default:
      return "#9eb4ca";
  }
}

function RhythmHeatmapPanel({ profile }: { profile: RhythmProfile | null }) {
  if (!profile) {
    return (
      <div className="cwp-rhythm-empty">正在分析你的工作节律…</div>
    );
  }

  const maxActive = Math.max(
    1,
    ...profile.hourBuckets.map((bucket) => bucket.activeSeconds),
  );

  return (
    <div className="cwp-rhythm-panel">
      <div className="cwp-rhythm-section">
        <div className="cwp-rhythm-section-title">每日时段节律</div>
        <div className="cwp-rhythm-heatmap">
          {profile.hourBuckets.map((bucket) => {
            const intensity = bucket.activeSeconds / maxActive;
            const isPeak = profile.peakHours.includes(bucket.index);
            return (
              <div
                key={bucket.index}
                className={"cwp-rhythm-cell" + (isPeak ? " is-peak" : "")}
                style={{
                  background: `rgba(92, 223, 255, ${0.08 + intensity * 0.85})`,
                }}
                title={`${bucket.index}:00 — 活跃 ${formatDuration(bucket.activeSeconds)} / ${bucket.sampleDays} 天`}
              >
                <span className="cwp-rhythm-cell-label">
                  {String(bucket.index).padStart(2, "0")}
                </span>
              </div>
            );
          })}
        </div>
        <div className="cwp-rhythm-axis">
          <span>00</span>
          <span>06</span>
          <span>12</span>
          <span>18</span>
          <span>23</span>
        </div>
      </div>

      <div className="cwp-rhythm-section">
        <div className="cwp-rhythm-section-title">每周分布</div>
        <div className="cwp-rhythm-weekday">
          {profile.weekdayBuckets.map((bucket, index) => {
            const weekdayMax = Math.max(
              1,
              ...profile.weekdayBuckets.map((b) => b.activeSeconds),
            );
            const ratio = bucket.activeSeconds / weekdayMax;
            const labels = ["一", "二", "三", "四", "五", "六", "日"];
            return (
              <div key={index} className="cwp-rhythm-weekday-row">
                <span className="cwp-rhythm-weekday-label">{labels[index]}</span>
                <div className="cwp-rhythm-weekday-bar-track">
                  <div
                    className="cwp-rhythm-weekday-bar"
                    style={{ width: `${Math.max(2, ratio * 100)}%` }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="cwp-rhythm-summary">{profile.summary}</div>
    </div>
  );
}
