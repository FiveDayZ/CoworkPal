import { invoke } from "@tauri-apps/api/core";
import type { MainRoute } from "../routeTypes";
import type {
  DailyWorkAssessment,
  DailyWorkAssessmentSummary,
  DailyWorkAssessmentTrend,
} from "../types/dailyWorkAssessment";
import type {
  AchievementCard,
  AchievementSummary,
  WeeklyGoals,
  TrackAchievementEventRequest,
  TrackAchievementEventResponse,
} from "../types/achievement";
import type { HardwareSnapshot } from "../types/hardware";
import type { AppSettings, AppSettingsPatch } from "../types/settings";
import type { WorkLogReport } from "../types/workLog";
import type { FocusSessionBook } from "../types/focus";
import { estimateFocusReward, type RewardAmount } from "../types/rewards";
import type { RhythmProfile } from "../types/rhythm";
import {
  TREND_RANGE_VALUES,
  type HealthTrendReport,
  type TrendRange,
} from "../types/health";
import type { NoteBook, NoteColor, NoteKind } from "../types/notes";
import type { TodaySuggestions } from "../types/suggestions";
import type { CloudSyncResult, SyncConfig, TokenRequestResult } from "../types/cloudSync";
import {
  defaultModuleLevels,
  type WorkshopModuleKey,
  type WorkshopProductionBreakdown,
  type WorkshopState,
  type WorkshopUpgradeQuotes,
} from "../types/workshop";

export type CoCatInteractionAction = "pet" | "sortParts";

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const browserSettings: AppSettings = {
  schemaVersion: 6,
  catName: "CoCat",
  onboardingVersion: 0,
  primaryMetric: "Cpu",
  launchAtStartup: false,
  isCatVisible: true,
  isMonitorBarVisible: false,
  enableSleepMode: true,
  enableSound: false,
  enableNotifications: false,
  isProductionPaused: false,
  enableLowPowerMode: false,
  enableStaticCatMode: false,
  enablePetBubble: true,
  showMonitorDataInTaskbar: false,
  samplingIntervalMs: 2000,
  backgroundSamplingIntervalMs: 5000,
  dataSortingCpuThreshold: 40,
  catSize: 1,
  catOpacity: 0.95,
  catWindowX: 1200,
  catWindowY: 520,
  monitorBarX: 580,
  monitorBarY: 24,
  cpuTemperatureWarning: 80,
  gpuTemperatureWarning: 82,
  memoryCrowdedThreshold: 82,
  errorGlitchCpuThreshold: 96,
  themeName: "coworkpal",
  visibleMonitorMetrics: ["Cpu", "Ram", "Gpu", "Network"],
  monitorBarMode: "Default",
  visibleTaskbarMetrics: ["Cpu", "Ram", "Gpu", "Network"],
  taskbarMonitorMode: "Default",
  catId: "DEV_CAT_ID",
  memoryReleaseEnabled: true,
  memoryAutoReleaseEnabled: false,
  memoryAutoReleaseThresholdGib: 8,
  memoryLastRelease: null,
  integratedHardwareMonitorEnabled: false,
};

const browserWorkshop: WorkshopState = {
  schemaVersion: 3,
  parts: 280,
  insight: 12,
  workshopLevel: 1,
  catAffinityLevel: 1,
  moduleLevels: defaultModuleLevels,
  lastProductionTime: 0,
  totalOnlineSeconds: 0,
  todayParts: 0,
  todayInsight: 0,
  lastDailyResetDate: "1970-01-01",
  affinityExperience: 42,
  completedOrderCount: 3,
  activeOrders: [
    {
      id: "preview:standard",
      kind: "standard",
      title: "基础校准单",
      description: "交付常规零件与灵感，完成今日基础维护。",
      requiredParts: 80,
      requiredInsight: 5,
      rewardAffinity: 18,
      createdAt: Date.now(),
      expiresAt: null,
    },
    {
      id: "preview:timed",
      kind: "timed",
      title: "限时加急工单",
      description: "在有效期内完成加急装配，获得更多亲密度。",
      requiredParts: 120,
      requiredInsight: 9,
      rewardAffinity: 32,
      createdAt: Date.now(),
      expiresAt: Date.now() + 8 * 60 * 60 * 1000,
    },
  ],
  completedOrderIds: [],
  lastOrderRefreshDate: "1970-01-01",
  rewardReceipts: {},
};

const browserSnapshot: HardwareSnapshot = {
  timestamp: Date.now(),
  cpuUsagePercent: null,
  gpuUsagePercent: null,
  memoryUsagePercent: null,
  cpuTemperatureCelsius: null,
  gpuTemperatureCelsius: null,
  diskReadBytesPerSecond: null,
  diskWriteBytesPerSecond: null,
  networkDownloadBytesPerSecond: null,
  networkUploadBytesPerSecond: null,
  cpuName: null,
  gpuName: null,
  gpuMemoryUsedBytes: null,
  gpuMemoryTotalBytes: null,
  totalMemoryBytes: null,
  usedMemoryBytes: null,
  cpuPhysicalCoreCount: null,
  cpuLogicalCoreCount: null,
  cpuTemperatureSource: null,
  deviceInventory: {
    motherboard: [],
    memoryModules: [],
    gpus: [],
    displays: [],
    disks: [],
    audioDevices: [],
    networkAdapters: [],
  },
  processes: [],
};

const browserAchievements: AchievementCard[] = [
  {
    achievementId: "A001",
    title: "第一次唤醒 CoCat",
    categoryKey: "daily_use",
    difficultyKey: "entry",
    points: 5,
    badgeKey: "cwp_badge_daily_first_launch_entry",
    isHidden: false,
    isUnlocked: true,
    unlockedAt: Date.now() - 86400000 * 5,
    unlockSnapshot: { "app.launch.count": 1 },
    conditionSummary: "app.launch.count >= 1",
    progress: null,
  },
  {
    achievementId: "A002",
    title: "30 分钟陪伴",
    categoryKey: "daily_use",
    difficultyKey: "entry",
    points: 5,
    badgeKey: "cwp_badge_daily_30m_companion_entry",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "lifetime.total_online_seconds >= 1800",
    progress: {
      current: 900,
      target: 1800,
      percent: 50,
      label: "分钟",
      isComplete: false,
    },
  },
  {
    achievementId: "A021",
    title: "4 小时陪伴",
    categoryKey: "daily_use",
    difficultyKey: "normal",
    points: 10,
    badgeKey: "cwp_badge_daily_4h_companion_normal",
    isHidden: false,
    isUnlocked: true,
    unlockedAt: Date.now() - 86400000 * 3,
    unlockSnapshot: { "lifetime.total_online_seconds": 14400 },
    conditionSummary: "lifetime.total_online_seconds >= 14400",
    progress: null,
  },
  {
    achievementId: "A022",
    title: "三日有迹",
    categoryKey: "long_streak",
    difficultyKey: "normal",
    points: 10,
    badgeKey: "cwp_badge_streak_3_active_days_normal",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "calendar.days(active_seconds >= 1800) >= 3",
    progress: {
      current: 1,
      target: 3,
      percent: 33.3,
      label: "天",
      isComplete: false,
    },
  },
  {
    achievementId: "A041",
    title: "24 小时陪伴",
    categoryKey: "daily_use",
    difficultyKey: "skilled",
    points: 20,
    badgeKey: "cwp_badge_daily_24h_companion_skilled",
    isHidden: false,
    isUnlocked: true,
    unlockedAt: Date.now() - 86400000,
    unlockSnapshot: { "lifetime.total_online_seconds": 86400 },
    conditionSummary: "lifetime.total_online_seconds >= 86400",
    progress: null,
  },
  {
    achievementId: "A042",
    title: "14 个活跃日",
    categoryKey: "long_streak",
    difficultyKey: "skilled",
    points: 20,
    badgeKey: "cwp_badge_streak_14_active_days_skilled",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "calendar.days(active_seconds >= 1800) >= 14",
    progress: {
      current: 5,
      target: 14,
      percent: 35.7,
      label: "天",
      isComplete: false,
    },
  },
  {
    achievementId: "A061",
    title: "7 天累计陪伴",
    categoryKey: "daily_use",
    difficultyKey: "elite",
    points: 35,
    badgeKey: "cwp_badge_daily_7d_companion_elite",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "lifetime.total_online_seconds >= 604800",
    progress: {
      current: 259200,
      target: 604800,
      percent: 42.8,
      label: "秒",
      isComplete: false,
    },
  },
  {
    achievementId: "A081",
    title: "30 天累计陪伴",
    categoryKey: "daily_use",
    difficultyKey: "epic",
    points: 60,
    badgeKey: "cwp_badge_daily_30d_companion_epic",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "lifetime.total_online_seconds >= 2592000",
    progress: {
      current: 864000,
      target: 2592000,
      percent: 33.3,
      label: "秒",
      isComplete: false,
    },
  },
  {
    achievementId: "A101",
    title: "365 天累计陪伴",
    categoryKey: "daily_use",
    difficultyKey: "legendary",
    points: 100,
    badgeKey: "cwp_badge_daily_365d_companion_legendary",
    isHidden: false,
    isUnlocked: false,
    unlockedAt: null,
    unlockSnapshot: null,
    conditionSummary: "lifetime.total_online_seconds >= 31536000",
    progress: {
      current: 3153600,
      target: 31536000,
      percent: 10.0,
      label: "秒",
      isComplete: false,
    },
  }
];

const browserAchievementSummary: AchievementSummary = {
  totalPoints: 35,
  unlockedCount: 3,
  visibleTotalCount: 9,
  hiddenUnlockedCount: 0,
  hiddenTotalCount: 0,
  byDifficulty: {
    entry: { unlocked: 1, total: 2 },
    normal: { unlocked: 1, total: 2 },
    skilled: { unlocked: 1, total: 2 },
    elite: { unlocked: 0, total: 1 },
    epic: { unlocked: 0, total: 1 },
    legendary: { unlocked: 0, total: 1 },
  },
  byCategory: {
    daily_use: { unlocked: 3, total: 7 },
    task_efficiency: { unlocked: 0, total: 0 },
    long_streak: { unlocked: 0, total: 2 },
    feature_exploration: { unlocked: 0, total: 0 },
    data_milestone: { unlocked: 0, total: 0 },
    workshop_growth: { unlocked: 0, total: 0 },
    hardware_health: { unlocked: 0, total: 0 },
    social_collaboration: { unlocked: 0, total: 0 },
    hidden_easter: { unlocked: 0, total: 0 },
  },
  latestUnlocks: [],
  highestRankUnlocks: [],
  pendingNotificationCount: 0,
};


function todayKey() {
  return new Date().toISOString().slice(0, 10);
}

function browserIsoWeekKey(date = new Date()) {
  const thursday = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
  const day = thursday.getUTCDay() || 7;
  thursday.setUTCDate(thursday.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(thursday.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((thursday.getTime() - yearStart.getTime()) / 86_400_000 + 1) / 7);
  return `${thursday.getUTCFullYear()}-W${String(week).padStart(2, "0")}`;
}

function relativeDateKey(offsetDays: number) {
  const date = new Date();
  date.setDate(date.getDate() - offsetDays);
  return date.toISOString().slice(0, 10);
}

function createBrowserWorkLogReport(date = todayKey()): WorkLogReport {
  return {
    date,
    totalScore: date === todayKey() ? 42 : 0,
    summary:
      date === todayKey()
        ? "浏览器预览数据：Tauri 运行后会基于真实硬件采样自动生成日志。"
        : "暂无足够数据：保持 CoCat 运行后，将自动生成当天工作投入度。",
    activeSeconds: date === todayKey() ? 3600 : 0,
    sampleCount: date === todayKey() ? 60 : 0,
    dimensions: [
      {
        key: "duration",
        title: "CoCat 运行时长",
        score: date === todayKey() ? 5 : 0,
        maxScore: 30,
        value: date === todayKey() ? "1.0h" : "0.0h",
        explanation: "按当日 CoCat 连续在线与生产观察时长折算。",
        facts: [
          { label: "运行时长", value: date === todayKey() ? "1h 0m" : "0h 0m" },
          { label: "采样记录", value: date === todayKey() ? "60 次" : "0 次" },
        ],
      },
      {
        key: "load",
        title: "硬件负载强度",
        score: date === todayKey() ? 12 : 0,
        maxScore: 25,
        value: "CPU 42% / RAM 60% / GPU 0%",
        explanation: "综合平均负载与 CPU>50%、RAM>70%、GPU>70% 的持续时长。",
        facts: [
          { label: "CPU>50%", value: "0h 18m" },
          { label: "RAM>70%", value: "0h 6m" },
          { label: "GPU>70%", value: "0h 0m" },
        ],
      },
      {
        key: "complexity",
        title: "任务复杂度",
        score: date === todayKey() ? 7 : 0,
        maxScore: 20,
        value: "高负载 18% / IO 10%",
        explanation: "结合高负载比例、磁盘读写总量、网络上传与访问流量。",
        facts: [
          { label: "磁盘读", value: "320.0 MB" },
          { label: "磁盘写", value: "96.0 MB" },
          { label: "上传", value: "48.0 MB" },
          { label: "访问", value: "220.0 MB" },
        ],
      },
      {
        key: "stability",
        title: "运行稳定性",
        score: date === todayKey() ? 14 : 0,
        maxScore: 15,
        value: "压力 20%",
        explanation: "高温持续时间越短，说明高强度工作下系统越稳定。",
        facts: [
          { label: "CPU>80C", value: "0h 0m" },
          { label: "GPU>80C", value: "0h 0m" },
        ],
      },
      {
        key: "continuity",
        title: "连续工作投入",
        score: date === todayKey() ? 4 : 0,
        maxScore: 10,
        value: date === todayKey() ? "点击 420 / 按键 1680" : "点击 0 / 按键 0",
        explanation: "结合持续观察窗口、鼠标点击和键盘按键次数，体现实际操作密度。",
        facts: [
          { label: "鼠标点击", value: date === todayKey() ? "420 次" : "0 次" },
          { label: "键盘按键", value: date === todayKey() ? "1680 次" : "0 次" },
        ],
      },
    ],
  };
}

function createBrowserWorkprint(dayType: "stableMaintenance" | "buildBurst" | "deepFocus" | "unknown") {
  const isEmpty = dayType === "unknown";
  const label =
    dayType === "buildBurst"
      ? "构建峰值型"
      : dayType === "deepFocus"
        ? "长时稳定型"
        : dayType === "stableMaintenance"
          ? "平稳维护型"
          : "数据积累中";

  return {
    label,
    description: isEmpty
      ? "这一天的数据还不足以形成稳定指纹。"
      : `今日 Workprint 呈现为${label}：负载、输入节奏和 IO 活动都来自预览数据。`,
    pixelGrid: isEmpty
      ? Array.from({ length: 64 }, () => 0)
      : [
          1, 1, 2, 2, 0, 0, 0, 0,
          1, 2, 2, 2, 0, 0, 0, 0,
          1, 1, 1, 0, 0, 0, 0, 0,
          1, 1, 0, 0, 0, 0, 0, 0,
          1, 2, 2, 1, 0, 0, 0, 0,
          1, 1, 2, 1, 0, 0, 0, 0,
          1, 2, 1, 2, 0, 0, 0, 0,
          1, 1, 2, 2, 0, 0, 0, 0,
        ],
    width: 8,
    height: 8,
    loadShape: isEmpty ? 0 : 0.42,
    inputRhythm: isEmpty ? 0 : 0.36,
    ioIntensity: isEmpty ? 0 : 0.28,
    thermalPressure: isEmpty ? 0 : 0.12,
    continuity: isEmpty ? 0 : 0.44,
  };
}

function createBrowserDailyWorkAssessment(date = todayKey()): DailyWorkAssessment {
  const report = createBrowserWorkLogReport(date);
  const isToday = date === todayKey();
  const workprint = createBrowserWorkprint(isToday ? "stableMaintenance" : "unknown");
  const timeline = isToday
    ? [
        {
          startTime: "09:00",
          endTime: "10:15",
          kind: "steadyProgress" as const,
          intensity: 0.42,
          label: "稳定推进",
          description: "预览片段：系统负载和输入节奏比较平稳。",
        },
        {
          startTime: "10:15",
          endTime: "10:45",
          kind: "buildPeak" as const,
          intensity: 0.68,
          label: "构建高峰",
          description: "预览片段：CPU 和磁盘活动同时抬升。",
        },
        {
          startTime: "10:45",
          endTime: "11:30",
          kind: "archiveFlow" as const,
          intensity: 0.5,
          label: "资料归档",
          description: "预览片段：磁盘和网络流动较明显。",
        },
      ]
    : [];

  return {
    date,
    dayType: isToday ? "stableMaintenance" : "unknown",
    dayTypeTitle: isToday ? "平稳维护日" : "数据积累中",
    rarity: isToday
      ? {
          tier: "B",
          label: "B 级工况卡",
          score: 48,
          reason: "预览画像分 42，连续活跃 1 天，负载指数 42，稳定指数 88",
        }
      : {
          tier: "C",
          label: "C 级工况卡",
          score: 12,
          reason: "预览数据不足，先收藏为观察卡",
        },
    title: isToday
      ? {
          family: "steady",
          title: "冷静维护员",
          level: 2,
          progress: 3,
          nextLevelAt: 7,
        }
      : {
          family: "observe",
          title: "观察记录员",
          level: 1,
          progress: 1,
          nextLevelAt: 3,
        },
    cocatCommentary: isToday
      ? {
          tone: "encouragement",
          title: "CoCat 点评：节奏已经成型",
          body: "预览数据里这张卡偏稳定。工况解释：正式运行后 CoCat 会用真实硬件、历史基线和输入节奏判断异常来源。",
        }
      : {
          tone: "tease",
          title: "CoCat 吐槽：这张卡还在孵化",
          body: "这一天暂时没有足够采样，CoCat 只能先把它记成观察卡。工况解释：样本不足时不做异常判断。",
        },
    score: report.totalScore,
    cocatSummary: isToday
      ? "浏览器预览数据：CoCat 会在 Tauri 运行后根据真实硬件与键鼠节奏生成每日工作画像。"
      : "这一天暂时没有足够的本地采样数据，CoCat 还无法形成可靠画像。",
    badgeIds: isToday ? ["STEADY", "OBSERVE"] : ["OBSERVE"],
    workprint,
    baseline: {
      sampleDays: 0,
      activeSecondsDeltaRatio: 0,
      loadDeltaRatio: 0,
      ioDeltaRatio: 0,
      thermalDeltaRatio: 0,
      inputDeltaRatio: 0,
      metrics: [
        {
          key: "active",
          label: "陪伴",
          currentValue: "2h 0m",
          baselineValue: "--",
          deltaRatio: 0,
          tone: "neutral",
        },
        {
          key: "load",
          label: "火力",
          currentValue: "42%",
          baselineValue: "--",
          deltaRatio: 0,
          tone: "neutral",
        },
        {
          key: "io",
          label: "IO",
          currentValue: "18%",
          baselineValue: "--",
          deltaRatio: 0,
          tone: "neutral",
        },
        {
          key: "thermal",
          label: "温度",
          currentValue: "24%",
          baselineValue: "--",
          deltaRatio: 0,
          tone: "neutral",
        },
        {
          key: "input",
          label: "输入",
          currentValue: "1200/h",
          baselineValue: "--",
          deltaRatio: 0,
          tone: "neutral",
        },
      ],
      summary: "浏览器预览暂不包含历史基线；Tauri 运行几天后会生成近 7 日对比。",
    },
    timeline,
    mvpSegments: timeline.slice(1, 3),
    highlights: [
      {
        title: "CoCat 已准备记录节奏",
        body: "真实运行后，这里会展示今天最有代表性的工作片段。",
        severity: "positive",
        metricValue: isToday ? "Preview" : null,
      },
    ],
    risks: [
      {
        title: "暂无明显隐患",
        body: "浏览器预览没有真实温度和内存压力数据，正式运行后会按本机采样判断。",
        severity: "neutral",
        metricValue: null,
      },
    ],
    suggestions: [
      {
        title: "保持 CoCat 常驻",
        body: "让 CoCat 安静观察一段时间后，每日工况报告会更贴近你的实际工作节奏。",
        severity: "neutral",
        metricValue: null,
      },
    ],
    processInsights: isToday
      ? [
          {
            name: "Code.exe",
            observedSeconds: 3540,
            activeSeconds: 1800,
            sampleCount: 59,
            activeSampleCount: 30,
            cpuPressurePercent: 18,
            averageMemoryBytes: 880 * 1024 * 1024,
            memoryBytesPeak: 1120 * 1024 * 1024,
            diskReadBytesTotal: 180 * 1024 * 1024,
            diskWriteBytesTotal: 92 * 1024 * 1024,
            rankLabel: "长驻后台",
            summary: "长驻后台：驻留 0h 59m，活跃 0h 30m，CPU 压力约 18%，内存峰值 1.1 GB。",
            severity: "positive",
            category: "开发工具",
            impact: "是当天主要活跃工具",
            recommendation: "把同类工作集中处理，可减少切换成本。",
            evidence: "活跃 30m · CPU 18% · 峰值内存 1.1 GB · 磁盘 272 MB",
          },
          {
            name: "chrome.exe",
            observedSeconds: 3300,
            activeSeconds: 1260,
            sampleCount: 55,
            activeSampleCount: 21,
            cpuPressurePercent: 11,
            averageMemoryBytes: 1340 * 1024 * 1024,
            memoryBytesPeak: 1800 * 1024 * 1024,
            diskReadBytesTotal: 68 * 1024 * 1024,
            diskWriteBytesTotal: 24 * 1024 * 1024,
            rankLabel: "内存常驻",
            summary: "内存常驻：驻留 0h 55m，活跃 0h 21m，CPU 压力约 11%，内存峰值 1.8 GB。",
            severity: "neutral",
            category: "浏览器",
            impact: "形成明显内存常驻",
            recommendation: "用完后关闭闲置窗口或标签页，给主要任务保留内存余量。",
            evidence: "活跃 21m · CPU 11% · 峰值内存 1.8 GB · 磁盘 92 MB",
          },
          {
            name: "node.exe",
            observedSeconds: 1680,
            activeSeconds: 900,
            sampleCount: 28,
            activeSampleCount: 15,
            cpuPressurePercent: 44,
            averageMemoryBytes: 420 * 1024 * 1024,
            memoryBytesPeak: 520 * 1024 * 1024,
            diskReadBytesTotal: 360 * 1024 * 1024,
            diskWriteBytesTotal: 160 * 1024 * 1024,
            rankLabel: "CPU 压力源",
            summary: "CPU 压力源：驻留 0h 28m，活跃 0h 15m，CPU 压力约 44%，内存峰值 520.0 MB。",
            severity: "warning",
            category: "编译构建",
            impact: "持续推高 CPU 压力",
            recommendation: "非必要时暂停后台任务，或把重负载操作安排在专注时段之外。",
            evidence: "活跃 15m · CPU 44% · 峰值内存 520 MB · 磁盘 520 MB",
          },
        ]
      : [],
    dimensions: report.dimensions.map((dimension) => ({
      ...dimension,
      title: mapAssessmentDimensionTitle(dimension.key, dimension.title),
    })),
  };
}

function createBrowserDailyWorkAssessmentHistory(
  limit = 14,
): DailyWorkAssessmentSummary[] {
  const summaries: DailyWorkAssessmentSummary[] = [
    {
      date: relativeDateKey(0),
      dayType: "stableMaintenance",
      dayTypeTitle: "平稳维护日",
      rarity: { tier: "B", label: "B 级工况卡", score: 48, reason: "预览稳定工况卡" },
      title: {
        family: "steady",
        title: "冷静维护员",
        level: 2,
        progress: 3,
        nextLevelAt: 7,
      },
      workprint: createBrowserWorkprint("stableMaintenance"),
      score: 42,
      cocatSummary: "今天的预览画像偏平稳：有持续观察，也有几段轻量推进。",
      badgeIds: ["STEADY", "OBSERVE"],
      hasTimeline: true,
      hasData: true,
    },
    {
      date: relativeDateKey(1),
      dayType: "buildBurst",
      dayTypeTitle: "编译构建日",
      rarity: { tier: "S", label: "S 级工况卡", score: 76, reason: "预览构建峰值卡" },
      title: {
        family: "build",
        title: "风暴构筑师",
        level: 2,
        progress: 3,
        nextLevelAt: 7,
      },
      workprint: createBrowserWorkprint("buildBurst"),
      score: 78,
      cocatSummary: "昨天出现过更明显的 CPU 与磁盘高峰，像是一段集中构建窗口。",
      badgeIds: ["BUILD", "HEAT"],
      hasTimeline: true,
      hasData: true,
    },
    {
      date: relativeDateKey(2),
      dayType: "deepFocus",
      dayTypeTitle: "深度工作日",
      rarity: { tier: "S", label: "S 级工况卡", score: 82, reason: "预览深度专注卡" },
      title: {
        family: "focus",
        title: "深潜构筑师",
        level: 2,
        progress: 3,
        nextLevelAt: 7,
      },
      workprint: createBrowserWorkprint("deepFocus"),
      score: 84,
      cocatSummary: "长时间稳定输入，压力适中，更像专注推进的一天。",
      badgeIds: ["FOCUS", "FLOW"],
      hasTimeline: true,
      hasData: true,
    },
    {
      date: relativeDateKey(3),
      dayType: "unknown",
      dayTypeTitle: "未形成画像",
      rarity: { tier: "C", label: "C 级工况卡", score: 0, reason: "预览空日期" },
      title: {
        family: "observe",
        title: "观察记录员",
        level: 1,
        progress: 1,
        nextLevelAt: 3,
      },
      workprint: createBrowserWorkprint("unknown"),
      score: 0,
      cocatSummary: "这一天没有足够的预览采样，历史墙会以灰态保留这个空日期。",
      badgeIds: ["EMPTY"],
      hasTimeline: false,
      hasData: false,
    },
    {
      date: relativeDateKey(4),
      dayType: "lowLoadCompanion",
      dayTypeTitle: "低负载陪伴日",
      rarity: { tier: "C", label: "C 级工况卡", score: 32, reason: "预览低负载陪伴卡" },
      title: {
        family: "quiet",
        title: "安静陪伴员",
        level: 1,
        progress: 1,
        nextLevelAt: 3,
      },
      workprint: createBrowserWorkprint("stableMaintenance"),
      score: 24,
      cocatSummary: "这天记录较轻，适合和高投入日期放在一起对照节奏变化。",
      badgeIds: ["LIGHT", "OBSERVE"],
      hasTimeline: false,
      hasData: true,
    },
  ];

  return summaries.slice(0, Math.max(1, limit));
}

function createBrowserDailyWorkAssessmentTrend(
  limit = 14,
): DailyWorkAssessmentTrend {
  const summaries = createBrowserDailyWorkAssessmentHistory(limit).filter(
    (item) => item.hasData,
  );
  const scoreSum = summaries.reduce((sum, item) => sum + item.score, 0);
  const averageScore = Math.round(scoreSum / Math.max(1, summaries.length));
  const best = summaries.reduce((current, item) =>
    item.score > current.score ? item : current,
  );
  const newestScore = summaries[0]?.score ?? 0;
  const oldestScore = summaries[summaries.length - 1]?.score ?? newestScore;
  const scoreDelta = newestScore - oldestScore;
  const timelineDays = summaries.filter((item) => item.hasTimeline).length;
  const dominantDayType = summaries[0]?.dayType ?? "unknown";
  const dominantDayTypeTitle = summaries[0]?.dayTypeTitle ?? "数据积累中";

  return {
    sampleDays: summaries.length,
    averageScore,
    bestDate: best?.date ?? null,
    bestScore: best?.score ?? null,
    bestDayType: best?.dayType ?? "unknown",
    bestDayTypeTitle: best?.dayTypeTitle ?? "数据积累中",
    dominantDayType,
    dominantDayTypeTitle,
    timelineDays,
    scoreDelta,
    summary:
      summaries.length > 0
        ? `浏览器预览：近 ${summaries.length} 个有记录日里，平均画像分 ${averageScore}，最常见的是${dominantDayTypeTitle}。`
        : "CoCat 还没有足够的历史日报来判断近期节奏。",
    insights: [
      {
        title: "近期主导形态",
        body: `预览数据里最常见的是${dominantDayTypeTitle}，正式运行后会按真实历史计算。`,
        severity: "positive",
        metricValue: dominantDayTypeTitle,
      },
      {
        title: "最高画像日",
        body: best
          ? `${best.date} 的预览画像分最高，为 ${best.score} 分。`
          : "暂无可比较日期。",
        severity: "positive",
        metricValue: best ? `${best.score} 分` : null,
      },
      {
        title: "节奏线覆盖",
        body: `当前预览中有 ${timelineDays}/${summaries.length} 天带有节奏线。`,
        severity: "neutral",
        metricValue: `${timelineDays}/${summaries.length} 天`,
      },
    ],
  };
}

function mapAssessmentDimensionTitle(key: string, fallback: string) {
  switch (key) {
    case "duration":
      return "陪伴时长";
    case "load":
      return "工坊火力";
    case "complexity":
      return "蓝图复杂度";
    case "stability":
      return "机器健康";
    case "continuity":
      return "专注节奏";
    default:
      return fallback;
  }
}

export async function getHardwareSnapshot(): Promise<HardwareSnapshot> {
  if (!isTauriRuntime()) {
    return { ...browserSnapshot, timestamp: Date.now() };
  }

  return invoke<HardwareSnapshot>("get_hardware_snapshot");
}

export async function trackAchievementEvent(
  request: TrackAchievementEventRequest,
): Promise<TrackAchievementEventResponse> {
  if (!isTauriRuntime()) {
    return { accepted: true, unlocked: [] };
  }

  return invoke<TrackAchievementEventResponse>("track_achievement_event", {
    request: {
      ...request,
      source: request.source ?? "frontend",
    },
  });
}

export async function getAchievementSummary(): Promise<AchievementSummary> {
  if (!isTauriRuntime()) {
    return browserAchievementSummary;
  }

  return invoke<AchievementSummary>("get_achievement_summary");
}

export async function getWeeklyGoals(): Promise<WeeklyGoals> {
  if (!isTauriRuntime()) {
    startPreviewClock();
    const weekKey = browserIsoWeekKey();
    const badgeKey = (achievementId: string) =>
      browserAchievements.find((achievement) => achievement.achievementId === achievementId)
        ?.badgeKey ?? "cwp_badge_daily_first_launch_entry";
    const qualified = browserFocusBook.sessions.filter((session) =>
      session.status === "completed" && session.plannedDurationSeconds >= 1500
      && session.creditedDurationMs >= session.plannedDurationSeconds * 1000
      && session.endedAt != null && browserIsoWeekKey(new Date(session.endedAt)) === weekKey).length;
    const orderCount = browserWorkshop.completedOrderIds.filter((id) => {
      const date = new Date(`${id.split(":")[0]}T12:00:00`);
      return Number.isFinite(date.getTime()) && browserIsoWeekKey(date) === weekKey;
    }).length;
    const goals: WeeklyGoals = {
      weekKey,
      bonusReward: { parts: 160, insight: 10, affinityExperience: 10 },
      bonusPaid: false,
      goals: [
        {
          goalId: "companion-120m",
          title: "本周陪伴 120 分钟",
          badgeKey: badgeKey("A002"),
          routeKey: "dashboard",
          current: Math.min(7200, browserWeeklySeconds),
          target: 7200,
          percent: Math.min(100, browserWeeklySeconds / 72),
          progressLabel: `${Math.floor(browserWeeklySeconds / 60)}/120 分钟`,
          isComplete: browserWeeklySeconds >= 7200,
          reward: { parts: 80, insight: 5, affinityExperience: 0 },
          rewardPaid: false,
        },
        {
          goalId: "qualified-focus-2",
          title: "完整完成 2 次专注",
          badgeKey: badgeKey("A066"),
          routeKey: "focus",
          current: Math.min(2, qualified),
          target: 2,
          percent: Math.min(100, qualified * 50),
          progressLabel: `${Math.min(2, qualified)}/2 次`,
          isComplete: qualified >= 2,
          reward: { parts: 160, insight: 10, affinityExperience: 5 },
          rewardPaid: false,
        },
        {
          goalId: "weekly-action",
          title: "2 天报告或 1 张工单",
          badgeKey: badgeKey("A003"),
          routeKey: "workLog",
          current: Math.min(1, orderCount),
          target: 1,
          percent: orderCount ? 100 : 0,
          progressLabel: `报告 0/2 天 · 工单 ${Math.min(1, orderCount)}/1 张`,
          isComplete: orderCount >= 1,
          reward: { parts: 80, insight: 5, affinityExperience: 5 },
          rewardPaid: false,
        },
      ],
    };
    for (const goal of goals.goals) {
      const key = `week:${weekKey}:${goal.goalId}`;
      if (goal.isComplete) grantPreviewReward(key, goal.title, goal.reward);
      goal.rewardPaid = key in browserWorkshop.rewardReceipts;
    }
    const bonusKey = `week:${weekKey}:all`;
    if (goals.goals.every((goal) => goal.isComplete)) {
      grantPreviewReward(bonusKey, "本周目标全部完成", goals.bonusReward);
    }
    goals.bonusPaid = bonusKey in browserWorkshop.rewardReceipts;
    return goals;
  }

  return invoke<WeeklyGoals>("get_weekly_goals");
}

export async function listAchievements(
  includeUnlockedHidden = true,
): Promise<AchievementCard[]> {
  if (!isTauriRuntime()) {
    return browserAchievements;
  }

  return invoke<AchievementCard[]>("list_achievements", {
    includeUnlockedHidden,
  });
}

export async function getAchievementDetail(
  achievementId: string,
): Promise<AchievementCard | null> {
  if (!isTauriRuntime()) {
    return (
      browserAchievements.find(
        (achievement) =>
          achievement.achievementId.toLowerCase() === achievementId.toLowerCase(),
      ) ?? null
    );
  }

  return invoke<AchievementCard | null>("get_achievement_detail", {
    achievementId,
  });
}

export async function markAchievementNotificationsSeen(
  unlockIds?: string[],
): Promise<AchievementSummary> {
  if (!isTauriRuntime()) {
    return { ...browserAchievementSummary, pendingNotificationCount: 0 };
  }

  return invoke<AchievementSummary>("mark_achievement_notifications_seen", {
    unlockIds: unlockIds ?? null,
  });
}

export async function getAppSettings(): Promise<AppSettings> {
  if (!isTauriRuntime()) {
    return { ...browserSettings };
  }

  return invoke<AppSettings>("get_app_settings");
}

export async function updateAppSettings(
  patch: AppSettingsPatch,
): Promise<AppSettings> {
  if (!isTauriRuntime()) {
    Object.assign(browserSettings, patch);
    return { ...browserSettings };
  }

  return invoke<AppSettings>("update_app_settings", { patch });
}

export async function getWorkshopState(): Promise<WorkshopState> {
  if (!isTauriRuntime()) {
    return browserWorkshop;
  }

  return invoke<WorkshopState>("get_workshop_state");
}

export async function rewardCoCatInteraction(
  action: CoCatInteractionAction,
): Promise<WorkshopState> {
  if (!isTauriRuntime()) {
    return { ...browserWorkshop };
  }

  return invoke<WorkshopState>("reward_cocat_interaction", { action });
}

export async function getWorkLogReport(date?: string): Promise<WorkLogReport> {
  if (!isTauriRuntime()) {
    return createBrowserWorkLogReport(date);
  }

  return invoke<WorkLogReport>("get_work_log_report", { date: date ?? null });
}

export async function getDailyWorkAssessment(
  date?: string,
): Promise<DailyWorkAssessment> {
  if (!isTauriRuntime()) {
    return createBrowserDailyWorkAssessment(date);
  }

  return invoke<DailyWorkAssessment>("get_daily_work_assessment", {
    date: date ?? null,
  });
}

export async function getDailyWorkAssessmentHistory(
  limit?: number,
): Promise<DailyWorkAssessmentSummary[]> {
  if (!isTauriRuntime()) {
    return createBrowserDailyWorkAssessmentHistory(limit);
  }

  return invoke<DailyWorkAssessmentSummary[]>(
    "get_daily_work_assessment_history",
    {
      limit: limit ?? null,
    },
  );
}

export async function getDailyWorkAssessmentTrend(
  limit?: number,
): Promise<DailyWorkAssessmentTrend> {
  if (!isTauriRuntime()) {
    return createBrowserDailyWorkAssessmentTrend(limit);
  }

  return invoke<DailyWorkAssessmentTrend>("get_daily_work_assessment_trend", {
    limit: limit ?? null,
  });
}

export async function toggleProductionPaused(): Promise<AppSettings> {
  return invoke<AppSettings>("toggle_production_paused");
}

const browserFocusBook: FocusSessionBook = { schemaVersion: 1, sessions: [] };
let previewClock: ReturnType<typeof setInterval> | null = null;
let browserWeeklySeconds = 0;
let previewWeek = browserIsoWeekKey();
let previewTickAt = Date.now();

function localDateKey(timestamp = Date.now()): string {
  const date = new Date(timestamp);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function grantPreviewReward(key: string, title: string, amount: RewardAmount) {
  if (key in browserWorkshop.rewardReceipts) return;
  const reward = { ...amount };
  browserWorkshop.parts += reward.parts;
  browserWorkshop.insight += reward.insight;
  browserWorkshop.todayParts += reward.parts;
  browserWorkshop.todayInsight += reward.insight;
  browserWorkshop.affinityExperience += reward.affinityExperience;
  browserWorkshop.catAffinityLevel += Math.floor(browserWorkshop.affinityExperience / 100);
  browserWorkshop.affinityExperience %= 100;
  browserWorkshop.rewardReceipts[key] = { title, reward, earnedAt: Date.now(), paidAt: Date.now() };
  window.dispatchEvent(new CustomEvent("preview:workshop", { detail: { ...browserWorkshop } }));
  if (reward.parts || reward.insight || reward.affinityExperience) {
    window.dispatchEvent(new CustomEvent("preview:reward", { detail: { rewardId: key, title, reward } }));
  }
}

function finishPreviewFocus(session: FocusSessionBook["sessions"][number]) {
  session.status = "completed";
  session.endedAt = Date.now();
  session.focusQuality = Math.max(0.4, 1 - session.distractionCount * 0.15);
  session.productionMultiplier = 1;
  const reward = estimateFocusReward(Math.floor(session.creditedDurationMs / 1000), session.distractionCount);
  const affinityToday = Object.entries(browserWorkshop.rewardReceipts).filter(([key, receipt]) =>
    key.startsWith("focus:") && receipt.reward.affinityExperience > 0 && localDateKey(receipt.earnedAt) === localDateKey()).length;
  if (session.plannedDurationSeconds >= 1500 && session.creditedDurationMs >= session.plannedDurationSeconds * 1000 && affinityToday < 2) {
    reward.affinityExperience = 5;
  }
  grantPreviewReward(`focus:${session.id}`, "专注完成奖励", reward);
  session.reward = browserWorkshop.rewardReceipts[`focus:${session.id}`].reward;
  session.rewardPaid = true;
  session.achievementRecorded = true;
}

function tickPreviewClock() {
  const now = Date.now();
  const delta = now - previewTickAt;
  previewTickAt = now;
  if (previewWeek !== browserIsoWeekKey()) {
    previewWeek = browserIsoWeekKey();
    browserWeeklySeconds = 0;
  }
  if (delta >= 0 && delta <= 30_000) browserWeeklySeconds += delta / 1000;
  for (const session of browserFocusBook.sessions.filter((item) => item.status === "active")) {
    const gap = session.lastTickAt == null ? 0 : now - session.lastTickAt;
    session.lastTickAt = now;
    if (gap >= 0 && gap <= 30_000) {
      session.creditedDurationMs = Math.min(session.plannedDurationSeconds * 1000, session.creditedDurationMs + gap);
    }
    if (session.creditedDurationMs >= session.plannedDurationSeconds * 1000) finishPreviewFocus(session);
  }
  window.dispatchEvent(new CustomEvent("preview:focus", { detail: cloneFocusBook() }));
}

function startPreviewClock() {
  if (previewClock == null) previewClock = setInterval(tickPreviewClock, 1000);
}

function cloneFocusBook(): FocusSessionBook {
  return {
    ...browserFocusBook,
    sessions: browserFocusBook.sessions.map((session) => ({ ...session })),
  };
}

export async function getFocusSessions(): Promise<FocusSessionBook> {
  if (!isTauriRuntime()) {
    return cloneFocusBook();
  }
  return invoke<FocusSessionBook>("get_focus_sessions");
}

export async function startFocusSession(
  taskLabel: string,
  durationMinutes: number,
): Promise<FocusSessionBook> {
  if (!isTauriRuntime()) {
    const task = taskLabel.trim();
    if (!task) {
      throw new Error("任务名称不能为空");
    }
    const now = Date.now();
    if (browserFocusBook.sessions.some((session) => session.status === "active")) {
      throw new Error("已有进行中的专注，请先完成或放弃");
    }
    startPreviewClock();
    browserFocusBook.sessions.push({
      id: `focus-preview-${now}`,
      taskLabel: task,
      plannedDurationSeconds: Math.min(180, Math.max(5, durationMinutes)) * 60,
      startedAt: now,
      endedAt: null,
      status: "active",
      distractionCount: 0,
      focusQuality: 0,
      productionMultiplier: 1.5,
      rewardVersion: 1,
      creditedDurationMs: 0,
      lastTickAt: now,
      reward: null,
      rewardPaid: false,
      achievementRecorded: false,
    });
    return cloneFocusBook();
  }
  return invoke<FocusSessionBook>("start_focus_session", {
    taskLabel,
    durationMinutes,
  });
}

export async function completeFocusSession(
  sessionId: string,
): Promise<[FocusSessionBook, WorkshopState]> {
  if (!isTauriRuntime()) {
    const session = browserFocusBook.sessions.find(
      (candidate) => candidate.id === sessionId && candidate.status !== "abandoned",
    );
    if (!session) {
      throw new Error("未找到进行中的专注会话");
    }
    if (session.status === "active") {
      tickPreviewClock();
      if (session.status === "active") finishPreviewFocus(session);
    }
    return [cloneFocusBook(), { ...browserWorkshop }];
  }
  return invoke<[FocusSessionBook, WorkshopState]>("complete_focus_session", {
    sessionId,
  });
}

export async function abandonFocusSession(
  sessionId: string,
): Promise<FocusSessionBook> {
  if (!isTauriRuntime()) {
    const session = browserFocusBook.sessions.find(
      (candidate) => candidate.id === sessionId && candidate.status === "active",
    );
    if (!session) {
      throw new Error("未找到进行中的专注会话");
    }
    session.status = "abandoned";
    session.endedAt = Date.now();
    session.productionMultiplier = 1;
    return cloneFocusBook();
  }
  return invoke<FocusSessionBook>("abandon_focus_session", { sessionId });
}

// --- Notes & memos ---

/** Browser-preview fallback: an empty note book. */
const emptyNoteBook: NoteBook = { schemaVersion: 1, notes: [] };

export async function getNotes(): Promise<NoteBook> {
  if (!isTauriRuntime()) {
    return { ...emptyNoteBook };
  }
  return invoke<NoteBook>("get_notes");
}

export async function createNote(args: {
  kind: NoteKind;
  title: string;
  body: string;
  memoDueAt: number | null;
  color: NoteColor;
}): Promise<[NoteBook, string]> {
  if (!isTauriRuntime()) {
    // Browser preview: fabricate a stable-ish id so callers that read it work.
    return [{ ...emptyNoteBook }, `note-preview-${Date.now()}`];
  }
  return invoke<[NoteBook, string]>("create_note", args);
}

export async function updateNote(args: {
  id: string;
  title: string;
  body: string;
  memoDueAt: number | null;
  color: NoteColor;
}): Promise<NoteBook> {
  if (!isTauriRuntime()) {
    return { ...emptyNoteBook };
  }
  return invoke<NoteBook>("update_note", args);
}

export async function toggleNotePinned(id: string): Promise<NoteBook> {
  if (!isTauriRuntime()) {
    return { ...emptyNoteBook };
  }
  return invoke<NoteBook>("toggle_note_pinned", { id });
}

export async function toggleNoteArchived(id: string): Promise<NoteBook> {
  if (!isTauriRuntime()) {
    return { ...emptyNoteBook };
  }
  return invoke<NoteBook>("toggle_note_archived", { id });
}

export async function deleteNote(id: string): Promise<NoteBook> {
  if (!isTauriRuntime()) {
    return { ...emptyNoteBook };
  }
  return invoke<NoteBook>("delete_note", { id });
}

/**
 * Export a note as a `.md` file. Shows a native save dialog. Returns the
 * chosen path on success, or null if the user cancelled the dialog.
 * (Browser preview: no-op, returns null.)
 */
export async function exportNote(id: string): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<string | null>("export_note", { id });
}

/**
 * Import a `.md` file as a new note. Shows a native open dialog; the file
 * content becomes the note body, and the title is derived from the first H1
 * heading or the filename. Returns the new note's id on success, or null if
 * the user cancelled the dialog. (Browser preview: no-op, returns null.)
 */
export async function importNote(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<string | null>("import_note");
}

export async function getRhythmProfile(): Promise<RhythmProfile> {
  if (!isTauriRuntime()) {
    return {
      hourBuckets: Array.from({ length: 24 }, (_, index) => ({
        index,
        activeSeconds: 0,
        avgFocusScore: 0,
        sampleDays: 0,
      })),
      weekdayBuckets: Array.from({ length: 7 }, (_, index) => ({
        index,
        activeSeconds: 0,
        avgFocusScore: 0,
        sampleDays: 0,
      })),
      peakHours: [],
      summary: "浏览器预览模式下暂无节律数据。",
      bestWorkWindow: null,
      lowEnergyWindow: null,
      interruptionSource: "浏览器预览模式下暂无中断数据。",
      sampleDays: 0,
      confidence: "低",
    };
  }
  return invoke<RhythmProfile>("get_rhythm_profile");
}

/** Browser-preview fallback: an empty-but-well-shaped report so the UI renders. */
function createBrowserHealthTrend(range: TrendRange): HealthTrendReport {
  const days = range === "days7" ? 7 : range === "days90" ? 90 : 30;
  return {
    windowDays: 0,
    range,
    scoreSeries: Array.from({ length: days }, (_, i) => ({
      date: `2026-01-${String(i + 1).padStart(2, "0")}`,
      totalScore: 0,
      durationScore: 0,
      loadScore: 0,
      complexityScore: 0,
      stabilityScore: 0,
      continuityScore: 0,
      activeSeconds: 0,
      hasData: false,
    })),
    averages: {
      score: 0,
      activeHours: 0,
      cpuAvg: 0,
      memoryAvg: 0,
      thermalAvg: 0,
      highLoadRatio: 0,
    },
    peaks: {
      bestScoreDate: null,
      bestScore: null,
      longestDayDate: null,
      longestHours: null,
      hottestDayDate: null,
      hottestThermal: null,
    },
    weekdayBreakdown: Array.from({ length: 7 }, (_, weekday) => ({
      weekday,
      avgScore: 0,
      avgHours: 0,
      sampleDays: 0,
    })),
    streaks: { current: 0, longest: 0, totalActiveDays: 0 },
    deltaVsPrev: { scoreDelta: 0, hoursDelta: 0, tone: "neutral" },
    healthScore: 0,
    healthGrade: "C",
    summary: "浏览器预览模式下暂无健康趋势数据。",
  };
}

export async function getHealthTrend(
  range: TrendRange = "days30",
): Promise<HealthTrendReport> {
  if (!isTauriRuntime()) {
    return createBrowserHealthTrend(range);
  }
  return invoke<HealthTrendReport>("get_health_trend", {
    range: TREND_RANGE_VALUES[range],
  });
}

/** Browser-preview fallback: no suggestions in the browser. */
function createBrowserTodaySuggestions(): TodaySuggestions {
  return {
    date: new Date().toISOString().slice(0, 10),
    top: [],
    all: [],
    generatedAt: Date.now(),
  };
}

export async function getTodaySuggestions(): Promise<TodaySuggestions> {
  if (!isTauriRuntime()) {
    return createBrowserTodaySuggestions();
  }
  return invoke<TodaySuggestions>("get_today_suggestions");
}

export async function showMainWindow(): Promise<void> {
  return invoke("show_main_window");
}

export async function showMainRoute(route: MainRoute): Promise<void> {
  return invoke("show_main_route", { route });
}

export async function hideMainWindow(): Promise<void> {
  return invoke("hide_main_window");
}

export async function showPetWindow(): Promise<void> {
  return invoke("show_pet_window");
}

export async function hidePetWindow(): Promise<void> {
  return invoke("hide_pet_window");
}

export async function toggleMonitorBar(): Promise<void> {
  return invoke("toggle_monitor_bar");
}

export async function showMonitorBar(): Promise<void> {
  return invoke("show_monitor_bar");
}

export async function hideMonitorBar(): Promise<void> {
  return invoke("hide_monitor_bar");
}

export async function togglePetPanel(): Promise<void> {
  return invoke("toggle_pet_panel");
}

export async function showPetPanel(): Promise<void> {
  return invoke("show_pet_panel");
}

export async function hidePetPanel(): Promise<void> {
  return invoke("hide_pet_panel");
}

export async function saveWindowPosition(
  windowLabel: "pet" | "monitor-bar",
  x: number,
  y: number,
): Promise<AppSettings> {
  return invoke<AppSettings>("save_window_position", { windowLabel, x, y });
}

export async function exitApp(): Promise<void> {
  return invoke("exit_app");
}

export async function getWorkshopUpgradeQuotes(): Promise<WorkshopUpgradeQuotes | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<WorkshopUpgradeQuotes>("get_workshop_upgrade_quotes");
}

export async function getWorkshopProductionBreakdown(): Promise<WorkshopProductionBreakdown> {
  if (!isTauriRuntime()) {
    const level = browserWorkshop.catAffinityLevel;
    const affinityMultiplier = 1 + Math.min(0.1, Math.max(0, level - 1) * 0.005);
    const affinity = level >= 20
      ? { title: "最佳拍档", tier: "bonded" as const, nextLevel: null, nextTitle: null }
      : level >= 10
        ? { title: "可靠拍档", tier: "partner" as const, nextLevel: 20, nextTitle: "最佳拍档" }
        : level >= 5
          ? { title: "默契搭档", tier: "trusted" as const, nextLevel: 10, nextTitle: "可靠拍档" }
          : { title: "初识搭档", tier: "new" as const, nextLevel: 5, nextTitle: "默契搭档" };
    return {
      partsPerMinute: 2.8 * affinityMultiplier,
      insightPerMinute: 0.24 * affinityMultiplier,
      partsActivity: 0.56,
      insightActivity: 0.48,
      workshopMultiplier: 1,
      partsModuleMultiplier: 1,
      insightModuleMultiplier: 1,
      stabilityMultiplier: 0.96,
      focusMultiplier: 1,
      affinityMultiplier,
      affinityTitle: affinity.title,
      affinityTier: affinity.tier,
      nextAffinityLevel: affinity.nextLevel,
      nextAffinityTitle: affinity.nextTitle,
    };
  }
  return invoke<WorkshopProductionBreakdown>("get_workshop_production_breakdown");
}

export async function completeWorkshopOrder(orderId: string): Promise<WorkshopState> {
  if (!isTauriRuntime()) {
    const order = browserWorkshop.activeOrders.find((candidate) => candidate.id === orderId);
    if (!order) throw new Error("工单不存在或已经完成");
    if (browserWorkshop.parts < order.requiredParts || browserWorkshop.insight < order.requiredInsight) {
      throw new Error("工坊资源不足");
    }
    browserWorkshop.parts -= order.requiredParts;
    browserWorkshop.insight -= order.requiredInsight;
    browserWorkshop.affinityExperience += order.rewardAffinity;
    while (browserWorkshop.affinityExperience >= 100) {
      browserWorkshop.affinityExperience -= 100;
      browserWorkshop.catAffinityLevel += 1;
    }
    browserWorkshop.completedOrderCount += 1;
    browserWorkshop.completedOrderIds.push(`${localDateKey()}:${orderId}`);
    browserWorkshop.activeOrders = browserWorkshop.activeOrders.filter(
      (order) => order.id !== orderId,
    );
    return { ...browserWorkshop };
  }
  return invoke<WorkshopState>("complete_workshop_order", { orderId });
}

export async function upgradeWorkshop(): Promise<WorkshopState> {
  return invoke<WorkshopState>("upgrade_workshop");
}

export async function upgradeWorkshopModule(
  moduleKey: WorkshopModuleKey,
  track: "parts" | "process",
): Promise<WorkshopState> {
  return invoke<WorkshopState>("upgrade_workshop_module", { moduleKey, track });
}

export async function resetWorkshopState(): Promise<WorkshopState> {
  if (!isTauriRuntime()) {
    Object.assign(browserWorkshop, {
      parts: 280,
      insight: 12,
      workshopLevel: 1,
      catAffinityLevel: 1,
      moduleLevels: structuredClone(defaultModuleLevels),
      lastProductionTime: Date.now(),
      totalOnlineSeconds: 0,
      todayParts: 0,
      todayInsight: 0,
      lastDailyResetDate: new Date().toISOString().split("T")[0],
      affinityExperience: 0,
      completedOrderCount: 0,
      activeOrders: [],
      completedOrderIds: [],
      lastOrderRefreshDate: "",
    });
    return { ...browserWorkshop };
  }
  return invoke<WorkshopState>("reset_workshop_state");
}

let browserSyncConfig: SyncConfig = {
  serverUrl: "",
  accessToken: "",
  userId: "",
  userName: "",
  tokenRequestId: "",
  tokenRequestSecret: "",
  tokenRequestKind: "",
  autoBackupEnabled: false,
  autoBackupIntervalMinutes: 30,
};

export async function getSyncConfig(): Promise<SyncConfig> {
  if (!isTauriRuntime()) return { ...browserSyncConfig };
  return invoke<SyncConfig>("get_sync_config");
}

export async function requestAccessToken(config: SyncConfig): Promise<TokenRequestResult> {
  if (!isTauriRuntime()) {
    throw new Error("令牌申请仅在桌面应用中可用");
  }
  return invoke<TokenRequestResult>("request_access_token", { config });
}

export async function requestAccessTokenRecovery(config: SyncConfig): Promise<TokenRequestResult> {
  if (!isTauriRuntime()) {
    throw new Error("令牌恢复仅在桌面应用中可用");
  }
  return invoke<TokenRequestResult>("request_access_token_recovery", { config });
}

export async function checkAccessTokenRequest(): Promise<TokenRequestResult> {
  if (!isTauriRuntime()) {
    throw new Error("令牌申请仅在桌面应用中可用");
  }
  return invoke<TokenRequestResult>("check_access_token_request");
}

export async function updateSyncConfig(config: SyncConfig): Promise<SyncConfig> {
  if (!isTauriRuntime()) {
    browserSyncConfig = { ...config };
    return { ...browserSyncConfig };
  }
  return invoke<SyncConfig>("update_sync_config", { config });
}

export async function uploadUserData(): Promise<CloudSyncResult> {
  if (!isTauriRuntime()) {
    throw new Error("云端同步仅在桌面应用中可用");
  }
  return invoke<CloudSyncResult>("upload_user_data");
}

export async function downloadUserData(): Promise<CloudSyncResult> {
  if (!isTauriRuntime()) {
    throw new Error("云端同步仅在桌面应用中可用");
  }
  return invoke<CloudSyncResult>("download_user_data");
}

import type { UpdateCheckResult } from "../types/update";

export async function checkUpdate(pat?: string): Promise<UpdateCheckResult> {
  if (!isTauriRuntime()) {
    return {
      hasUpdate: true,
      currentVersion: "1.0.1",
      latestVersion: "1.1.0",
      changelog: "### 更新日志\n- [优化] 提升了桌面猫咪动画运行效率\n- [修复] 解决任务栏嵌入在某些分辨率下的偏移问题",
      downloadUrl: "https://mock.com/cowork-pal_1.1.0_x64-setup.exe",
      assetId: 12345,
      assetSize: 15420100,
      assetName: "cowork-pal_1.1.0_x64-setup.exe",
    };
  }

  return invoke<UpdateCheckResult>("check_update", { pat });
}

export async function downloadUpdate(
  assetId: number,
  assetName: string,
  pat?: string,
): Promise<string> {
  if (!isTauriRuntime()) {
    return "C:\\MockPath\\cowork-pal_1.1.0_x64-setup.exe";
  }

  return invoke<string>("download_update", { assetId, assetName, pat });
}

export async function installUpdate(packagePath: string): Promise<void> {
  if (!isTauriRuntime()) {
    alert(`Mock安装：已经启动安装程序 ${packagePath}`);
    return;
  }

  return invoke<void>("install_update", { packagePath });
}

// ---------------------------------------------------------------------------
// Memory release module
// ---------------------------------------------------------------------------

export interface MemoryStatus {
  totalBytes: number;
  usedBytes: number;
  usedGib: number;
  loadPercent: number;
}

export interface MemoryReleaseResult {
  releasedBytes: number;
  fullTier: boolean;
  note: string;
}

/**
 * Live system memory snapshot for the settings card. Returns a mock reading in
 * browser preview mode so the UI can render the "current usage" line.
 */
export async function getMemoryStatus(): Promise<MemoryStatus> {
  if (!isTauriRuntime()) {
    // Mock: 38% of 16 GiB used.
    const total = 16 * 1024 * 1024 * 1024;
    const loadPercent = 38;
    const used = Math.round((total * loadPercent) / 100);
    return {
      totalBytes: total,
      usedBytes: used,
      usedGib: used / (1024 * 1024 * 1024),
      loadPercent,
    };
  }
  return invoke<MemoryStatus>("get_memory_status");
}

/**
 * Trigger a manual full-tier memory release. Shows a UAC prompt (Windows); on
 * decline the backend degrades to a light-tier sweep. Resolves with the result
 * and a human-facing note. The backend also emits `memory:release-completed`.
 */
export async function triggerMemoryRelease(): Promise<MemoryReleaseResult> {
  if (!isTauriRuntime()) {
    return {
      releasedBytes: 1200 * 1024 * 1024,
      fullTier: true,
      note: "浏览器预览：模拟释放 1.2 GB",
    };
  }
  return invoke<MemoryReleaseResult>("trigger_memory_release");
}

/**
 * Show the native context menu on the taskbar monitor window. The backend
 * builds the same menu the tray icon uses and pops it up via the native
 * TrackPopupMenuEx, so it isn't clipped by the 36px-tall embedded window.
 * No-op in browser preview (no Tauri runtime).
 */
export async function showTaskbarContextMenu(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  return invoke<void>("show_taskbar_context_menu");
}
