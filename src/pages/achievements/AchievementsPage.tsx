import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  getAchievementDetail,
  trackAchievementEvent,
} from "../../services/tauriCommands";
import { copyTextToClipboard } from "../../services/clipboard";
import { useAchievementStore } from "../../stores/achievementStore";
import type {
  AchievementCard,
  AchievementCategory,
  AchievementDifficulty,
} from "../../types/achievement";
import { badgeAssets } from "../../ui/assets";

type AchievementFilter = "all" | "unlocked" | "locked";
type DifficultyFilter = "all" | AchievementDifficulty;
type CategoryFilter = "all" | AchievementCategory;

const difficultyOptions: Array<{ key: DifficultyFilter; label: string }> = [
  { key: "all", label: "全部难度" },
  { key: "entry", label: "入门" },
  { key: "normal", label: "进阶" },
  { key: "skilled", label: "熟练" },
  { key: "elite", label: "精英" },
  { key: "epic", label: "史诗" },
  { key: "legendary", label: "传说" },
];

const categoryOptions: Array<{ key: CategoryFilter; label: string }> = [
  { key: "all", label: "全部分类" },
  { key: "daily_use", label: "日常" },
  { key: "task_efficiency", label: "效率" },
  { key: "long_streak", label: "打卡" },
  { key: "feature_exploration", label: "探索" },
  { key: "data_milestone", label: "数据" },
  { key: "workshop_growth", label: "工坊" },
  { key: "hardware_health", label: "健康" },
  { key: "social_collaboration", label: "协作" },
  { key: "hidden_easter", label: "隐藏" },
];

const unlockedOptions: Array<{ key: AchievementFilter; label: string }> = [
  { key: "all", label: "全部状态" },
  { key: "unlocked", label: "已解锁" },
  { key: "locked", label: "未解锁" },
];

export function AchievementsPage() {
  const summary = useAchievementStore((state) => state.summary);
  const cards = useAchievementStore((state) => state.cards);
  const loadSummary = useAchievementStore((state) => state.loadSummary);
  const loadCards = useAchievementStore((state) => state.loadCards);
  const markNotificationsSeen = useAchievementStore(
    (state) => state.markNotificationsSeen,
  );
  const [difficulty, setDifficulty] = useState<DifficultyFilter>("all");
  const [category, setCategory] = useState<CategoryFilter>("all");
  const [unlockedState, setUnlockedState] = useState<AchievementFilter>("all");
  const [selectedAchievementId, setSelectedAchievementId] = useState<string | null>(
    null,
  );
  const [selectedDetailCard, setSelectedDetailCard] =
    useState<AchievementCard | null>(null);
  const [exportHint, setExportHint] = useState<string | null>(null);
  const [viewedUnreadCount, setViewedUnreadCount] = useState<number | null>(null);
  const exportHintTimerRef = useRef<number | null>(null);

  useEffect(() => {
    let disposed = false;

    void (async () => {
      const loadedSummary = await loadSummary();
      await loadCards();

      if (disposed) {
        return;
      }

      const unreadCount = loadedSummary.pendingNotificationCount;
      setViewedUnreadCount((current) => Math.max(current ?? 0, unreadCount));

      if (unreadCount > 0) {
        await markNotificationsSeen();
      }
    })().catch((error) => {
        console.error("Failed to load achievement gallery", error);
      });

    return () => {
      disposed = true;
    };
  }, [loadCards, loadSummary, markNotificationsSeen]);

  useEffect(() => {
    void trackAchievementEvent({
      eventName: "achievement.gallery.view",
      occurredAt: Date.now(),
      idempotencyKey: `achievement.gallery.view:${Date.now()}`,
      payload: {
        filterCategory: category,
        filterDifficulty: difficulty,
        filterUnlockedState: unlockedState,
      },
      source: "achievement-page",
    }).catch((error) => {
      console.error("Failed to track achievement gallery view", error);
    });
  }, [category, difficulty, unlockedState]);

  useEffect(() => {
    let disposed = false;

    if (!selectedAchievementId) {
      setSelectedDetailCard(null);
      return undefined;
    }

    setSelectedDetailCard(
      cards.find((card) => card.achievementId === selectedAchievementId) ?? null,
    );
    void getAchievementDetail(selectedAchievementId)
      .then((card) => {
        if (!disposed && card) {
          setSelectedDetailCard(card);
        }
      })
      .catch((error) => {
        console.error("Failed to load achievement detail", error);
      });

    return () => {
      disposed = true;
    };
  }, [cards, selectedAchievementId]);

  useEffect(
    () => () => {
      if (exportHintTimerRef.current !== null) {
        window.clearTimeout(exportHintTimerRef.current);
      }
    },
    [],
  );

  const visibleCards = useMemo(
    () =>
      cards.filter((card) => {
        if (difficulty !== "all" && card.difficultyKey !== difficulty) {
          return false;
        }
        if (category !== "all" && card.categoryKey !== category) {
          return false;
        }
        if (unlockedState === "unlocked" && !card.isUnlocked) {
          return false;
        }
        if (unlockedState === "locked" && card.isUnlocked) {
          return false;
        }
        return true;
      }),
    [cards, category, difficulty, unlockedState],
  );
  const selectedCard =
    selectedDetailCard ??
    cards.find((card) => card.achievementId === selectedAchievementId) ??
    null;
  const totalAchievementCount =
    (summary?.visibleTotalCount ?? 0) + (summary?.hiddenTotalCount ?? 0);
  const newPromptCount = Math.max(
    viewedUnreadCount ?? 0,
    summary?.pendingNotificationCount ?? 0,
  );

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

  async function handleExportProfileSnapshot() {
    const now = Date.now();
    const unlockedCount = summary?.unlockedCount ?? 0;
    const totalPoints = summary?.totalPoints ?? 0;
    const text = [
      "CoreWorkPal 成就图鉴快照",
      `导出时间：${formatDateTime(now)}`,
      `成就点数：${totalPoints}`,
      `已解锁：${unlockedCount}/${totalAchievementCount}`,
      `隐藏成就：${summary?.hiddenUnlockedCount ?? 0}/${summary?.hiddenTotalCount ?? 0}`,
      `当前筛选：${visibleCards.length} 项`,
    ].join("\n");

    await copyTextToClipboard(text);
    await trackAchievementEvent({
      eventName: "share.profile_snapshot.export",
      occurredAt: now,
      idempotencyKey: `share.profile_snapshot.export:${now}`,
      payload: {
        format: "text",
        hiddenUnlockedCount: summary?.hiddenUnlockedCount ?? 0,
        totalAchievementCount,
        totalPoints,
        unlockedCount,
      },
      source: "achievement-page",
    });
    showExportHint("图鉴快照已复制");
  }

  async function handleExportAchievementCard(card: AchievementCard) {
    const now = Date.now();
    const text = [
      "CoreWorkPal 成就徽章卡",
      `成就：${card.title}`,
      `编号：${card.achievementId}`,
      `徽章资源：${card.badgeKey}`,
      `难度：${difficultyLabel(card.difficultyKey)}`,
      `分类：${categoryLabel(card.categoryKey)}`,
      `点数：+${card.points}`,
      `状态：${card.isUnlocked ? "已解锁" : "未解锁"}`,
      `触发条件：${formatConditionText(card)}`,
      `当前进度：${formatCardProgress(card)}`,
      `解锁时间：${card.unlockedAt ? formatDateTime(card.unlockedAt) : "尚未解锁"}`,
      `导出时间：${formatDateTime(now)}`,
    ].join("\n");

    await copyTextToClipboard(text);
    await trackAchievementEvent({
      eventName: "share.achievement_card.export",
      occurredAt: now,
      idempotencyKey: `share.achievement_card.export:${card.achievementId}:${now}`,
      payload: {
        achievementId: card.achievementId,
        badgeKey: card.badgeKey,
        format: "text",
        isUnlocked: card.isUnlocked,
      },
      source: "achievement-detail",
    });
    showExportHint("徽章卡已复制");
  }

  return (
    <div
      className={`cwp-page cwp-achievements-page${
        selectedCard ? " is-detail" : ""
      }`}
    >
      {selectedCard ? (
        <AchievementDetailPage
          card={selectedCard}
          exportHint={exportHint}
          onExport={handleExportAchievementCard}
          onBack={() => setSelectedAchievementId(null)}
        />
      ) : (
        <>
      <section className="cwp-achievements-overview" aria-label="成就统计">
        <SummaryTile
          label="已解锁"
          value={`${summary?.unlockedCount ?? 0}/${totalAchievementCount}`}
        />
        <SummaryTile
          label="隐藏成就"
          value={`${summary?.hiddenUnlockedCount ?? 0}/${summary?.hiddenTotalCount ?? 0}`}
        />
        <SummaryTile label="当前筛选" value={`${visibleCards.length} 项`} />
        <SummaryTile
          label="新解锁提示"
          value={`${newPromptCount}`}
        />
      </section>

      <section className="cwp-achievements-toolbar" aria-label="成就筛选">
        <FilterSelect
          label="难度"
          onChange={(value) => setDifficulty(value as DifficultyFilter)}
          options={difficultyOptions}
          value={difficulty}
        />
        <FilterSelect
          label="分类"
          onChange={(value) => setCategory(value as CategoryFilter)}
          options={categoryOptions}
          value={category}
        />
        <FilterSelect
          label="状态"
          onChange={(value) => setUnlockedState(value as AchievementFilter)}
          options={unlockedOptions}
          value={unlockedState}
        />
        <button
          className="cwp-achievements-action"
          onClick={() => {
            void handleExportProfileSnapshot().catch((error) => {
              console.error("Failed to export achievement profile snapshot", error);
              showExportHint("复制失败，请重试");
            });
          }}
          type="button"
        >
          复制图鉴快照
        </button>
        {exportHint ? (
          <span className="cwp-achievements-export-hint">{exportHint}</span>
        ) : null}
      </section>

      <section className="cwp-achievements-content">
        <div className="cwp-achievements-list" aria-label="成就列表">
          {visibleCards.length > 0 ? (
            visibleCards.map((card) => (
              <AchievementCardItem
                card={card}
                key={card.achievementId}
                onSelect={setSelectedAchievementId}
              />
            ))
          ) : (
            <div className="cwp-achievements-empty">
              当前筛选下没有可展示的成就。
            </div>
          )}
        </div>
      </section>
        </>
      )}
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="cwp-achievements-summary-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function FilterSelect({
  label,
  onChange,
  options,
  value,
}: {
  label: string;
  onChange: (value: string) => void;
  options: Array<{ key: string; label: string }>;
  value: string;
}) {
  return (
    <label className="cwp-achievements-filter">
      <span>{label}</span>
      <select onChange={(event) => onChange(event.target.value)} value={value}>
        {options.map((option) => (
          <option key={option.key} value={option.key}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function AchievementCardItem({
  card,
  onSelect,
}: {
  card: AchievementCard;
  onSelect: (achievementId: string) => void;
}) {
  return (
    <button
      className={`cwp-achievement-card${card.isUnlocked ? " is-unlocked" : ""}`}
      onClick={() => onSelect(card.achievementId)}
      type="button"
    >
      <BadgeImage card={card} />
      <div className="cwp-achievement-card-copy">
        <div className="cwp-achievement-card-title-row">
          <strong>{card.title}</strong>
          <span>{difficultyLabel(card.difficultyKey)}</span>
        </div>
        <p>{formatConditionText(card)}</p>
        <ProgressLine card={card} />
      </div>
      <div className="cwp-achievement-card-state">
        <strong>+{card.points}</strong>
        <span>{card.isUnlocked ? "已解锁" : "未解锁"}</span>
      </div>
    </button>
  );
}

function AchievementDetailPage({
  card,
  exportHint,
  onExport,
  onBack,
}: {
  card: AchievementCard;
  exportHint: string | null;
  onExport: (card: AchievementCard) => void;
  onBack: () => void;
}) {
  const snapshotEntries = Object.entries(card.unlockSnapshot ?? {})
    .sort(([left], [right]) => left.localeCompare(right))
    .slice(0, 12);

  return (
    <section className="cwp-achievement-detail-page">
      <div className="cwp-achievement-detail-nav">
        <div className="cwp-achievement-detail-actions">
          <button onClick={onBack} type="button">
            返回图鉴
          </button>
          <button
            onClick={() => {
              void Promise.resolve(onExport(card)).catch((error) => {
                console.error("Failed to export achievement card", error);
              });
            }}
            type="button"
          >
            复制徽章卡
          </button>
          {exportHint ? (
            <span className="cwp-achievements-export-hint">{exportHint}</span>
          ) : null}
        </div>
        <span>{card.achievementId}</span>
      </div>

      <aside className="cwp-achievement-detail">
        <div className="cwp-achievement-detail-hero">
          <BadgeImage card={card} />
          <div>
            <span>{card.isUnlocked ? "已解锁" : "未解锁"}</span>
            <strong>{card.title}</strong>
            <em>
              {difficultyLabel(card.difficultyKey)} · {categoryLabel(card.categoryKey)} · +
              {card.points}
            </em>
          </div>
        </div>

        <DetailSection title="触发条件">
          <p>{formatConditionText(card)}</p>
        </DetailSection>

        <DetailSection title="当前完成进度">
          {card.progress ? (
            <div className="cwp-achievement-detail-progress">
              <div className="cwp-achievement-card-progress-track">
                <span style={{ width: `${card.progress.percent}%` }} />
              </div>
              <p>
                {formatProgressLabel(card.progress.label)} ·{" "}
                {formatProgressValue(card.progress.current, card.progress.label)} /{" "}
                {formatProgressValue(card.progress.target, card.progress.label)}
              </p>
            </div>
          ) : (
            <p>{card.isUnlocked ? "已完成" : "暂无可展示进度"}</p>
          )}
        </DetailSection>

        <DetailSection title="成就解锁时间">
          <p>{card.unlockedAt ? formatDateTime(card.unlockedAt) : "尚未解锁"}</p>
        </DetailSection>

        <DetailSection title="解锁瞬间快照">
          {snapshotEntries.length > 0 ? (
            <div className="cwp-achievement-snapshot-list">
              {snapshotEntries.map(([key, value]) => (
                <div key={key}>
                  <span>{formatSnapshotLabel(key)}</span>
                  <strong>{formatProgressValue(value, key)}</strong>
                </div>
              ))}
            </div>
          ) : (
            <p>{card.isUnlocked ? "该成就暂无快照数据。" : "解锁后自动记录。"}</p>
          )}
        </DetailSection>
      </aside>
    </section>
  );
}

function DetailSection({
  children,
  title,
}: {
  children: ReactNode;
  title: string;
}) {
  return (
    <section className="cwp-achievement-detail-section">
      <span>{title}</span>
      {children}
    </section>
  );
}

function BadgeImage({ card }: { card: AchievementCard }) {
  return (
    <div className="cwp-achievement-card-badge">
      <img
        alt={card.title}
        className={`cwp-achievement-badge-image ${card.isUnlocked ? "" : "is-locked"}`}
        onError={(event) => {
          event.currentTarget.src = badgeAssets.cwp_badge_placeholder;
        }}
        src={badgeAssets[card.badgeKey] || badgeAssets.cwp_badge_placeholder}
      />
    </div>
  );
}

function ProgressLine({ card }: { card: AchievementCard }) {
  if (!card.progress) {
    return null;
  }

  return (
    <div className="cwp-achievement-card-progress">
      <div className="cwp-achievement-card-progress-track">
        <span style={{ width: `${card.progress.percent}%` }} />
      </div>
      <em>
        {formatProgressLabel(card.progress.label)} ·{" "}
        {formatProgressValue(card.progress.current, card.progress.label)} /{" "}
        {formatProgressValue(card.progress.target, card.progress.label)}
      </em>
    </div>
  );
}

function difficultyLabel(difficulty: AchievementDifficulty) {
  return (
    difficultyOptions.find((option) => option.key === difficulty)?.label ??
    difficulty
  );
}

function categoryLabel(category: AchievementCategory) {
  return (
    categoryOptions.find((option) => option.key === category)?.label ?? category
  );
}

function formatDateTime(timestamp: number) {
  return new Date(timestamp).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatConditionText(card: AchievementCard) {
  if (!card.conditionSummary) {
    return "隐藏条件会在解锁后显示。";
  }

  return formatReadableCondition(card.conditionSummary);
}

function formatReadableCondition(rawCondition: string) {
  const condition = rawCondition.replace(/`/g, "").trim().replace(/。$/, "");
  const exact = conditionTextOverrides[condition];
  if (exact) {
    return exact;
  }

  const parts = condition
    .split(/\s+且\s+|；|;/)
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length > 1) {
    return parts.map(formatReadableConditionPart).join("；");
  }

  return formatReadableConditionPart(condition);
}

function formatReadableConditionPart(condition: string) {
  const calendarDays = condition.match(/^calendar\.days\((.+)\)\s*>=\s*(\d+)$/);
  if (calendarDays) {
    return `累计 ${calendarDays[2]} 天满足：${formatReadablePredicate(calendarDays[1])}`;
  }

  const consecutiveDays = condition.match(
    /^calendar\.consecutive_days\((.+)\)\s*>=\s*(\d+)$/,
  );
  if (consecutiveDays) {
    return `连续 ${consecutiveDays[2]} 天满足：${formatReadablePredicate(
      consecutiveDays[1],
    )}`;
  }

  const calendarMonths = condition.match(
    /^calendar\.months\(report_generated_days\s*>=\s*(\d+)\)\s*>=\s*(\d+)$/,
  );
  if (calendarMonths) {
    return `累计 ${calendarMonths[2]} 个月，每个月至少生成 ${calendarMonths[1]} 份工况报告`;
  }

  const distinctCount = condition.match(/^distinct_count\((.+)\)\s*>=\s*(\d+)$/);
  if (distinctCount) {
    return formatDistinctCountCondition(distinctCount[1], Number(distinctCount[2]));
  }

  const maxCondition = condition.match(/^max\((.+)\)\s*>=\s*(\d+)$/);
  if (maxCondition) {
    return `任意一条${formatMetricGroup(maxCondition[1])}达到 ${maxCondition[2]} 级`;
  }

  const titleFamilyBucket = condition.match(/^7 个 `?title_family`? 每类等级全部 `?>=\s*(\d+)`?$/);
  if (titleFamilyBucket) {
    return `7 类工况职级全部达到 Lv.${titleFamilyBucket[1]}`;
  }

  const comparison = formatComparisonCondition(condition);
  if (comparison) {
    return comparison;
  }

  return condition
    .replace(/6 个模块的 parts 等级全部 >= (\d+)/, "6 个模块的零件等级全部达到 $1 级")
    .replace(/6 个模块的 process 等级全部 >= (\d+)/, "6 个模块的工艺等级全部达到 $1 级")
    .replace(/6 个模块的 parts 与 process 共 12 条轨道全部 >= (\d+)/, "6 个模块的零件与工艺共 12 条升级轨道全部达到 $1 级")
    .replace(/9 个分类中每个分类已解锁成就数 >= (\d+)/, "9 个分类中，每个分类至少解锁 $1 项成就")
    .replace(/7 个 report_day_type 每类天数全部 >= (\d+)，不含 unknown/, "7 类工作画像每天数都至少达到 $1 天，不包含未知类型")
    .replace(/18 个 CoreCat 动画状态的 corecat\.animation_seen\.count\(animationState\) 全部 >= (\d+)/, "18 种 CoreCat 动画状态，每种至少观看 $1 次")
    .replace(/6 个难度中每个难度已解锁成就数全部 >= (\d+)/, "6 个难度中，每个难度至少解锁 $1 项成就");
}

function formatComparisonCondition(condition: string) {
  const comparison = condition.match(/^(.+?)\s*(>=|<=|=)\s*([0-9.]+)$/);
  if (!comparison) {
    return null;
  }

  const [, leftExpression, operator, targetText] = comparison;
  const target = Number(targetText);
  const label = formatMetricExpression(leftExpression);
  const value = formatProgressValue(target, leftExpression);

  if (operator === "<=") {
    return `${label}不超过 ${value}`;
  }
  if (operator === "=") {
    return `${label}等于 ${value}`;
  }
  return `${label}达到 ${value}`;
}

function formatReadablePredicate(predicate: string) {
  return predicate
    .split(/\s+AND\s+/)
    .map((part) => {
      const normalized = part.trim();
      const dayType = normalized.match(/^report_day_type\s*=\s*'([^']+)'$/);
      if (dayType) {
        return `工作画像为「${dayTypeLabel(dayType[1])}」`;
      }

      const comparison = formatComparisonCondition(normalized);
      return comparison ?? normalized;
    })
    .join("，并且");
}

function formatDistinctCountCondition(expression: string, target: number) {
  const normalized = expression.trim();
  if (normalized.startsWith("page.view.pageKey")) {
    return `查看过 ${target} 个主要页面`;
  }
  if (normalized === "settings.update.themeName") {
    return `使用过 ${target} 种不同主题`;
  }
  if (normalized === "workshop.module_upgrade.moduleKey") {
    return `升级过 ${target} 类不同工坊模块`;
  }
  if (normalized === "settings.update.changedKey") {
    return `调整过 ${target} 项不同设置`;
  }
  if (normalized.startsWith("worklog.daily_generated.dayType")) {
    return `收集到 ${target} 种不同工作画像`;
  }

  return `${formatMetricExpression(normalized)}达到 ${target} 项`;
}

function formatMetricGroup(expression: string) {
  if (expression.includes("workshop.module_level.parts")) {
    return "模块零件升级轨道";
  }
  if (expression.includes("workshop.module_level.process")) {
    return "模块工艺升级轨道";
  }
  return "升级轨道";
}

function formatMetricExpression(expression: string) {
  const parts = expression.split("+").map((part) => part.trim()).filter(Boolean);
  if (parts.length > 1) {
    return parts.map(formatMetricLabel).join("与") + "合计";
  }

  return formatMetricLabel(expression.trim());
}

function formatMetricLabel(metric: string) {
  const normalized = metric.trim();
  const pageView = normalized.match(/^page\.view\.count\(pageKey='([^']+)'\)$/);
  if (pageView) {
    return `查看「${pageLabel(pageView[1])}」页面次数`;
  }

  const settingUpdate = normalized.match(
    /^settings\.update\.count\(changedKey='([^']+)'(?:,\s*value=(true|false))?\)$/,
  );
  if (settingUpdate) {
    const suffix = settingUpdate[2] === "true" ? "开启次数" : "调整次数";
    return `${settingLabel(settingUpdate[1])}${suffix}`;
  }

  const animationSeen = normalized.match(
    /^corecat\.animation_seen\.count\(animationState='([^']+)'\)$/,
  );
  if (animationSeen) {
    return `看到「${animationLabel(animationSeen[1])}」动画次数`;
  }

  if (normalized.startsWith("pet.click_burst.count")) {
    return "2 秒内快速抚摸猫咪三连次数";
  }

  const titleLevel = normalized.match(/^worklog\.title_level\.([a-z]+)$/);
  if (titleLevel) {
    return `${titleFamilyLabel(titleLevel[1])}等级`;
  }

  const knownLabel = metricLabels[normalized];
  if (knownLabel) {
    return knownLabel;
  }

  const readable = normalized
    .replace(/^当前可见指标数$/, "当前可见监控指标数")
    .replace(/^report_generated_days$/, "每月工况报告天数")
    .replace(/^report_day_type$/, "工作画像类型")
    .replace(/\bparts\b/g, "零件")
    .replace(/\bprocess\b/g, "工艺");

  if (readable !== normalized || !/[.()_=']/.test(readable)) {
    return readable;
  }

  return humanizeMetricKey(readable);
}

function formatProgressLabel(label: string) {
  if (!label.includes(".") && !label.includes("(")) {
    return metricLabels[label] ?? label.replace("累计在线秒数", "累计陪伴时长");
  }

  return formatMetricExpression(label);
}

function formatSnapshotLabel(key: string) {
  return formatMetricLabel(key);
}

function formatProgressValue(value: number, context = "") {
  if (isRarityRankContext(context)) {
    return `${rarityRankLabel(value)} 级工况卡`;
  }
  if (isDurationContext(context)) {
    return formatDuration(value);
  }
  if (isByteContext(context)) {
    return formatBytes(value);
  }
  if (isCountContext(context)) {
    return `${formatNumber(value)} 次`;
  }
  if (context.includes("level") || context.includes("等级") || context.includes("工坊等级")) {
    return `${formatNumber(value)} 级`;
  }
  if (value >= 1024 * 1024 * 1024) {
    return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  }
  return formatNumber(value);
}

function formatNumber(value: number) {
  if (value >= 10000) {
    return Math.round(value).toLocaleString("zh-CN");
  }
  if (value % 1 !== 0) {
    return value.toFixed(1);
  }
  return `${value}`;
}

function formatDuration(seconds: number) {
  if (seconds >= 86_400) {
    const days = Math.floor(seconds / 86_400);
    const hours = Math.floor((seconds % 86_400) / 3_600);
    return hours > 0 ? `${days} 天 ${hours} 小时` : `${days} 天`;
  }
  if (seconds >= 3_600) {
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    return minutes > 0 ? `${hours} 小时 ${minutes} 分钟` : `${hours} 小时`;
  }
  if (seconds >= 60) {
    return `${Math.round(seconds / 60)} 分钟`;
  }
  return `${formatNumber(seconds)} 秒`;
}

function formatBytes(value: number) {
  const units = [
    { size: 1024 ** 4, label: "TiB" },
    { size: 1024 ** 3, label: "GiB" },
    { size: 1024 ** 2, label: "MiB" },
  ];
  const unit = units.find((item) => value >= item.size);
  if (!unit) {
    return `${formatNumber(value)} B`;
  }
  return `${(value / unit.size).toFixed(value >= 10 * unit.size ? 0 : 1)} ${unit.label}`;
}

function isDurationContext(context: string) {
  return /seconds|秒数|时长|陪伴/.test(context);
}

function isRarityRankContext(context: string) {
  return /rarity_rank|rarity\.max_rank|工况卡等级/.test(context);
}

function isByteContext(context: string) {
  return /bytes|数据|流量|网络|本地/.test(context);
}

function isCountContext(context: string) {
  return /count|次数|输入|按键|点击|报告|成就|导出|互动|页面|设置|动画/.test(context);
}

function pageLabel(pageKey: string) {
  return (
    {
      dashboard: "控制台",
      workshop: "工坊",
      devices: "设备",
      worklog: "日报",
      settings: "设置",
      about: "关于",
      achievements: "成就",
    }[pageKey] ?? pageKey
  );
}

function settingLabel(settingKey: string) {
  return (
    {
      showMonitorDataInTaskbar: "任务栏监控",
      visibleMonitorMetrics: "可见监控指标",
    }[settingKey] ?? settingKey
  );
}

function dayTypeLabel(dayType: string) {
  return (
    {
      deepFocus: "深度专注日",
      buildBurst: "构建爆发日",
      archiveFlow: "归档流转日",
      pressureRepair: "高压抢修日",
      stableMaintenance: "稳定维护日",
      fragmentedSwitching: "碎片切换日",
      lowLoadCompanion: "低负载陪伴日",
      unknown: "未知画像",
    }[dayType] ?? dayType
  );
}

function rarityRankLabel(rank: number) {
  if (rank >= 5) {
    return "SS";
  }
  if (rank >= 4) {
    return "S";
  }
  if (rank >= 3) {
    return "A";
  }
  if (rank >= 2) {
    return "B";
  }
  return "C";
}

function titleFamilyLabel(family: string) {
  return (
    {
      focus: "专注职级",
      build: "构建职级",
      archive: "归档职级",
      pressure: "高压修复师",
      steady: "稳定维护职级",
      switch: "多任务切换职级",
      quiet: "低负载陪伴职级",
      observe: "观察记录职级",
    }[family] ?? "工况职级"
  );
}

function animationLabel(animationState: string) {
  return (
    {
      errorGlitch: "错误故障",
      achievementPop: "成就弹出",
    }[animationState] ?? animationState
  );
}

function humanizeMetricKey(metric: string) {
  const words = metric
    .replace(/\([^)]*\)/g, "")
    .replace(/[._-]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => metricWordLabels[word] ?? word);

  return words.length > 0 ? words.join("") : "相关进度";
}

const metricWordLabels: Record<string, string> = {
  achievement: "成就",
  animation: "动画",
  app: "应用",
  card: "卡",
  changedKey: "设置项",
  click: "点击",
  count: "次数",
  daily: "每日",
  days: "天数",
  disk: "磁盘",
  enabled: "开启",
  export: "导出",
  generated: "生成",
  gallery: "图鉴",
  hardware: "硬件",
  hidden: "隐藏",
  high: "高",
  import: "导入",
  insight: "灵感",
  keyboard: "键盘",
  launch: "启动",
  lifetime: "累计",
  load: "负载",
  low: "低",
  memory: "内存",
  metric: "指标",
  mode: "模式",
  monitor: "监控",
  mouse: "鼠标",
  network: "网络",
  online: "在线",
  open: "打开",
  panel: "面板",
  part: "零件",
  parts: "零件",
  pet: "CoreCat",
  press: "按键",
  profile: "档案",
  report: "报告",
  score: "评分",
  seconds: "时长",
  setting: "设置",
  settings: "设置",
  share: "分享",
  snapshot: "快照",
  storage: "存档",
  taskbar: "任务栏",
  thermal: "温度",
  total: "总计",
  unlocked: "已解锁",
  update: "更新",
  view: "查看",
  warning: "警报",
  windowMs: "时间窗口",
  worklog: "日报",
  workshop: "工坊",
};

const metricLabels: Record<string, string> = {
  "app.launch.count": "启动 CoreWorkPal 次数",
  "lifetime.total_online_seconds": "累计陪伴时长",
  "worklog.daily_generated.count": "生成工况报告次数",
  "worklog.rarity.max_rank": "工况卡最高等级",
  "worklog.rarity.max_score": "工况卡最高稀有度分",
  "worklog.title_level.max": "最高工况职级等级",
  "settings.update.count": "保存设置次数",
  "monitor_bar.open.count": "打开悬浮监控条次数",
  "pet.panel.open.count": "打开 CoreCat 面板次数",
  "pet.click.count": "抚摸猫咪次数",
  "pet.drag_end.count": "搬动 CoreCat 次数",
  "workshop.module_upgrade.count": "工坊模块升级次数",
  "workshop.level_up.count": "工坊升级次数",
  "workshop.level": "工坊等级",
  "workshop.module.max_level": "模块最高等级",
  "lifetime.parts_earned": "累计零件",
  "lifetime.insight_earned": "累计灵感",
  "lifetime.keyboard_press_count": "键盘按键次数",
  "lifetime.mouse_click_count": "鼠标点击次数",
  "share.report_card.export.count": "报告卡导出次数",
  "share.achievement_card.export.count": "徽章卡导出次数",
  "share.profile_snapshot.export.count": "图鉴快照导出次数",
  "share.profile_snapshot.import.count": "外部快照导入次数",
  "achievement.gallery.view.count": "打开成就图鉴次数",
  "achievement.unlocked.count": "已解锁成就数量",
  "non_hidden_achievement.unlocked.count": "已解锁可见成就数量",
  "hidden_achievement.unlocked.count(excludeSelf=true)": "已解锁隐藏成就数量",
  "lifetime.high_load_seconds": "累计高负载时长",
  "lifetime.cpu_over_50_seconds": "CPU 负载超过 50% 的累计时长",
  "lifetime.memory_over_70_seconds": "内存占用超过 70% 的累计时长",
  "lifetime.gpu_over_70_seconds": "GPU 负载超过 70% 的累计时长",
  "lifetime.thermal_warning_seconds": "累计温度警报时长",
  "lifetime.low_power_mode_enabled_seconds": "低功耗模式累计开启时长",
  "lifetime.disk_bytes_total": "本地磁盘流转数据量",
  "lifetime.network_bytes_total": "网络流转数据量",
  "active_seconds": "当日陪伴时长",
  "high_load_seconds": "当日高负载时长",
  "thermal_warning_seconds": "当日温度警报时长",
  "active_00_05_seconds": "午夜时段陪伴时长",
  "low_power_mode_enabled_seconds": "当日低功耗模式开启时长",
  "report_score": "日报评分",
  "rarity_rank": "工况卡等级",
  "rarity_score": "工况卡稀有度分",
  "title_level": "工况职级等级",
  "title_progress": "同类职级累计天数",
  "storage.corruption_rebuilt.count": "存档自动修复次数",
  "累计在线秒数": "累计陪伴时长",
  "去重数量": "已收集不同项",
};

const conditionTextOverrides: Record<string, string> = {
  "settings.update.count(changedKey='visibleMonitorMetrics') >= 1 且当前可见指标数 >= 3":
    "至少调整过 1 次可见监控指标，并且当前至少展示 3 个监控指标",
  "6 个模块的 parts 等级全部 >= 5": "6 个模块的零件等级全部达到 5 级",
  "6 个模块的 process 等级全部 >= 5": "6 个模块的工艺等级全部达到 5 级",
  "6 个模块的 parts 等级全部 >= 20": "6 个模块的零件等级全部达到 20 级",
  "6 个模块的 process 等级全部 >= 20": "6 个模块的工艺等级全部达到 20 级",
  "6 个模块的 parts 等级全部 >= 50": "6 个模块的零件等级全部达到 50 级",
  "6 个模块的 process 等级全部 >= 50": "6 个模块的工艺等级全部达到 50 级",
  "6 个模块的 parts 与 process 共 12 条轨道全部 >= 100":
    "6 个模块的零件与工艺共 12 条升级轨道全部达到 100 级",
  "9 个分类中每个分类已解锁成就数 >= 5":
    "9 个成就分类中，每个分类至少解锁 5 项成就",
  "7 个 report_day_type 每类天数全部 >= 30，不含 unknown":
    "7 类工作画像每天数都至少达到 30 天，不包含未知画像",
  "18 个 CoreCat 动画状态的 corecat.animation_seen.count(animationState) 全部 >= 10":
    "18 种 CoreCat 动画状态，每种至少观看 10 次",
  "6 个难度中每个难度已解锁成就数全部 >= 10":
    "6 个难度中，每个难度至少解锁 10 项成就",
  "worklog.rarity.max_rank >= 2": "获得过至少 1 张 B 级或更高等级的工况卡",
  "worklog.rarity.max_rank >= 3": "获得过至少 1 张 A 级或更高等级的工况卡",
  "worklog.rarity.max_rank >= 4": "获得过至少 1 张 S 级或更高等级的工况卡",
  "worklog.rarity.max_rank >= 5": "获得过至少 1 张 SS 级工况卡",
  "calendar.days(rarity_rank >= 3) >= 7": "累计获得 7 天 A 级或更高等级的工况卡",
  "calendar.days(rarity_rank >= 4) >= 30": "累计获得 30 天 S 级或更高等级的工况卡",
  "worklog.title_level.pressure >= 2": "高压修复师职级达到 Lv.2",
  "worklog.title_level.pressure >= 3": "高压修复师职级达到 Lv.3",
  "worklog.title_level.max >= 3": "任意工况职级达到 Lv.3",
  "worklog.title_level.max >= 5": "任意工况职级达到 Lv.5",
  "7 个 title_family 每类等级全部 >= 3": "7 类工况职级全部达到 Lv.3",
  "7 个 title_family 每类等级全部 >= 5": "7 类工况职级全部达到 Lv.5",
};

function formatCardProgress(card: AchievementCard) {
  if (!card.progress) {
    return card.isUnlocked ? "已完成" : "暂无可展示进度";
  }

  return `${formatProgressLabel(card.progress.label)} ${formatProgressValue(
    card.progress.current,
    card.progress.label,
  )} / ${formatProgressValue(card.progress.target, card.progress.label)}`;
}
