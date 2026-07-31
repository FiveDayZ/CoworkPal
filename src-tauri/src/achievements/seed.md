# CoworkPal 成就与数字徽章系统开发文档

## 0. 文档边界

本文档用于后续严格落地 CoworkPal 专属成就与数字徽章系统。系统必须满足以下原则：

- 成就由本地后端事件埋点和规则引擎自动解锁，不允许普通运营或前端人工发放。
- 管理员补发仅作为数据损坏、迁移失败、规则缺陷修复后的兜底能力，必须写入审计表。
- 所有成就条件必须由次数、时长、数值、连续天数、去重数量或明确行为事件组成。
- 隐藏成就不在未解锁图鉴中预展示，满足条件后自动解锁并弹窗告知。
- 默认本地优先，不上传硬件明细、键鼠原始序列或窗口位置；如未来接入云同步，仅同步成就定义版本、解锁结果和匿名聚合计数。

核心术语：

| 术语 | 定义 |
| --- | --- |
| 成就定义 | 固定规则、难度、点数、分类、徽章资源占位符。 |
| 徽章 | 成就对应的数字徽章资源，本文只定义唯一名称，不设计图片。 |
| 事件 | 前端或 Rust 后端产生的可审计行为记录，例如 `pet.click`、`workshop.level_up`。 |
| 计数器 | 由事件归并得到的统计字段，例如累计在线秒数、连续打卡天数、模块升级次数。 |
| 解锁记录 | 某用户某成就首次满足条件后的永久记录。 |
| 隐藏成就 | 未解锁前不展示标题、描述、徽章，仅计入隐藏总数。 |

## ① 成就系统整体架构设计

### 1.1 系统定位

成就系统服务于 CoworkPal 的桌面伴侣、硬件监控、每日工况报告、工坊养成和 CoCat 互动体验。它不是运营活动系统，也不是排行榜系统。首版目标是：

- 让用户长期使用 CoworkPal 时形成明确的成长反馈。
- 将已有硬件采样、每日工况、工坊产出、窗口设置、CoCat 交互转化为可收集图鉴。
- 为前端提供总点数、按难度筛选、按分类筛选、隐藏成就解锁弹窗和个人主页陈列能力。
- 为后续统计留出事件表、日聚合表和规则版本字段。

### 1.2 推荐实现形态

CoworkPal 当前是 Tauri 2 + React 19 + TypeScript + Rust，并已有 `StorageService` 本地 JSON 存储。成就系统推荐新增本地 SQLite 数据库，原因是成就事件和计数器天然需要按时间、用户、事件名、规则版本查询。首版可使用 Rust `rusqlite` 或 `tauri-plugin-sql`，数据库文件建议放在现有 app data root 下：

```text
{app_data_root}/achievements.sqlite
```

若短期继续沿用 JSON，也必须保持本文表结构字段语义不变，后续迁移到 SQLite 时不得改变接口契约。

### 1.3 模块分层

```text
Rust 后端
├─ AchievementEventService
│  ├─ 接收前端 track 事件
│  ├─ 接收硬件采样、工坊、日报等后端内部事件
│  └─ 写入 achievement_events
├─ AchievementCounterService
│  ├─ 按事件更新 lifetime / daily / rolling / distinct 计数器
│  └─ 写入 achievement_progress_counters 与 achievement_daily_rollups
├─ AchievementRuleEngine
│  ├─ 读取 achievement_definitions
│  ├─ 按 condition_json 自动评估
│  └─ 写入 achievement_user_unlocks
├─ AchievementNotificationService
│  ├─ 生成待展示队列
│  ├─ emit achievement:unlocked
│  └─ 触发 cocat:interaction-state = achievementPop
└─ AchievementAdminRepairService
   └─ 仅补发、撤销错误补发、审计导出

React 前端
├─ achievementApi.ts
├─ achievementStore.ts
├─ AchievementGalleryPage
├─ AchievementToastQueue
├─ AchievementUnlockModal
└─ ProfileAchievementShelf
```

### 1.4 事件流

1. 前端或 Rust 后端产生事件。
2. Rust 调用 `record_achievement_event` 写入事件表，使用 `idempotency_key` 防重。
3. `AchievementCounterService` 根据事件名更新计数器和日聚合。
4. `AchievementRuleEngine` 只评估受该事件影响的成就定义。
5. 满足条件且未解锁时写入 `achievement_user_unlocks`。
6. 写入 `achievement_notification_queue`。
7. 向前端 emit `achievement:unlocked`，同时 emit `cocat:interaction-state` 为 `achievementPop`。
8. 前端按队列展示弹窗、声音、CoCat 动画和图鉴红点。

### 1.5 事件命名规范

事件名采用小写点分隔，格式为：

```text
{domain}.{object}.{action}
```

示例：

| 事件名 | 触发来源 | 关键 payload |
| --- | --- | --- |
| `app.launch` | Rust 启动完成 | `appVersion`、`bootMode` |
| `app.active_minute` | Rust 采样聚合 | `seconds` |
| `page.view` | 前端路由进入 | `pageKey` |
| `settings.update` | 设置保存成功 | `changedKeys`、`newValuesHash` |
| `pet.click` | CoCat 菜单「抚摸猫咪」 | `action` |
| `pet.drag_end` | PetWindow | `durationMs`、`distancePx` |
| `pet.panel.open` | Tauri command | `source` |
| `workshop.level_up` | Rust 工坊保存成功 | `fromLevel`、`toLevel` |
| `workshop.module_upgrade` | Rust 工坊保存成功 | `moduleKey`、`track`、`fromLevel`、`toLevel` |
| `workshop.production_tick` | Rust 产出 tick | `partsDelta`、`insightDelta` |
| `worklog.daily_generated` | 日报生成 | `date`、`score`、`dayType` |
| `hardware.segment_rollup` | 15 分钟聚合 | `highLoadSeconds`、`thermalWarningSeconds`、`ioBytes` |
| `share.report_card.export` | 前端导出 | `format` |
| `share.achievement_card.export` | 前端导出 | `achievementId`、`format` |
| `share.profile_snapshot.export` | 前端导出 | `format` |
| `achievement.gallery.view` | 前端页面进入 | `filterCategory`、`filterDifficulty` |
| `storage.corruption_rebuilt` | StorageService 修复 | `fileName` |
| `cocat.animation_seen` | CoCat 动画状态机 | `animationState` |

### 1.6 规则条件 DSL

成就条件存入 `condition_json`，首版支持以下操作：

```json
{
  "all": [
    { "counter": "lifetime.total_online_seconds", "op": ">=", "value": 1800 },
    { "counter": "calendar.active_days_ge_1800s", "op": ">=", "value": 3 }
  ]
}
```

支持节点：

| 节点 | 用途 |
| --- | --- |
| `all` | 全部满足。 |
| `any` | 任一满足。 |
| `counter` | 读取单个数值计数器。 |
| `distinct_count` | 读取去重数量，例如已访问页面数。 |
| `consecutive_days` | 读取连续天数计数器。 |
| `per_bucket_min` | 每个桶都达到阈值，例如 6 个模块均达到 50 级。 |
| `exclude_self` | 仅用于元成就，统计时排除当前成就。 |

禁止使用自然语言条件进入 `condition_json`。

## ② 难度分级权重规则定义

| 难度 | difficulty_key | 点数 | 权重 | 条件范围 | 展示色建议 |
| --- | --- | ---: | ---: | --- | --- |
| 入门（简单） | `entry` | 5 | 1 | 首次使用、单次行为、30 分钟内可达成。 | 铜色 |
| 进阶（普通） | `normal` | 10 | 2 | 3 到 7 天、低累计量、多页面基础探索。 | 青色 |
| 熟练（中等） | `skilled` | 20 | 4 | 7 到 30 天、中等累计量、功能组合使用。 | 蓝色 |
| 精英（困难） | `elite` | 35 | 7 | 30 到 90 天、高累计量、多行为叠加。 | 紫色 |
| 史诗 | `epic` | 60 | 12 | 90 到 180 天、端到端熟练使用、百万级或百日级积累。 | 金色 |
| 传说 | `legendary` | 100 | 20 | 180 天以上、全年级、全模块满级或跨维度长期积累。 | 虹彩 |

点数规则：

- 用户成就总点数 = 已解锁成就 `points` 求和。
- 隐藏成就点数计入总点数，但未解锁前不计入用户可见清单。
- 被禁用成就仍保留历史点数，除非该成就是规则错误导致的错误发放并通过管理员修复撤销。
- 重复达成不重复给点数；需要阶梯奖励时建立多个成就定义，例如 100、1000、10000。

难度约束：

- 入门必须只依赖 1 个主要条件。
- 进阶可以有 1 到 2 个条件。
- 熟练必须至少包含 1 个累计或连续条件。
- 精英必须至少包含 1 个长周期或多行为条件。
- 史诗必须至少包含 90 天级、百万级、满级前置或跨分类条件之一。
- 传说必须包含全年级、满级、千万级、全频段或跨 12 个月条件之一。

## ③ 数据库核心表结构设计

### 3.1 `achievement_definitions`

```sql
CREATE TABLE achievement_definitions (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  category_key TEXT NOT NULL,
  difficulty_key TEXT NOT NULL,
  points INTEGER NOT NULL,
  badge_key TEXT NOT NULL UNIQUE,
  is_hidden INTEGER NOT NULL DEFAULT 0,
  repeat_policy TEXT NOT NULL DEFAULT 'once',
  condition_json TEXT NOT NULL,
  enabled_from_version TEXT NOT NULL DEFAULT '0.1.9',
  enabled_to_version TEXT,
  definition_version INTEGER NOT NULL DEFAULT 1,
  display_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX idx_achievement_definitions_category
  ON achievement_definitions(category_key, difficulty_key);
```

### 3.2 `achievement_events`

```sql
CREATE TABLE achievement_events (
  event_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  event_name TEXT NOT NULL,
  occurred_at INTEGER NOT NULL,
  received_at INTEGER NOT NULL,
  source TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  app_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL DEFAULT 1,
  UNIQUE(user_id, idempotency_key)
);

CREATE INDEX idx_achievement_events_user_time
  ON achievement_events(user_id, occurred_at);

CREATE INDEX idx_achievement_events_name_time
  ON achievement_events(event_name, occurred_at);
```

### 3.3 `achievement_progress_counters`

```sql
CREATE TABLE achievement_progress_counters (
  user_id TEXT NOT NULL,
  counter_key TEXT NOT NULL,
  scope TEXT NOT NULL,
  scope_key TEXT NOT NULL,
  numeric_value REAL NOT NULL DEFAULT 0,
  text_value TEXT,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (user_id, counter_key, scope, scope_key)
);

CREATE INDEX idx_achievement_counters_key
  ON achievement_progress_counters(counter_key, numeric_value);
```

`scope` 可选值：

| scope | scope_key 示例 | 用途 |
| --- | --- | --- |
| `lifetime` | `all` | 生命周期累计。 |
| `daily` | `2026-06-24` | 单日聚合。 |
| `monthly` | `2026-06` | 月聚合。 |
| `rolling` | `last_7_days` | 滚动窗口。 |
| `distinct` | `page.view` | 去重集合计数，明细可放入 payload 或派生表。 |

### 3.4 `achievement_daily_rollups`

```sql
CREATE TABLE achievement_daily_rollups (
  user_id TEXT NOT NULL,
  date_key TEXT NOT NULL,
  active_seconds INTEGER NOT NULL DEFAULT 0,
  total_online_seconds INTEGER NOT NULL DEFAULT 0,
  high_load_seconds INTEGER NOT NULL DEFAULT 0,
  cpu_over_50_seconds INTEGER NOT NULL DEFAULT 0,
  memory_over_70_seconds INTEGER NOT NULL DEFAULT 0,
  gpu_over_70_seconds INTEGER NOT NULL DEFAULT 0,
  thermal_warning_seconds INTEGER NOT NULL DEFAULT 0,
  disk_bytes_total INTEGER NOT NULL DEFAULT 0,
  network_bytes_total INTEGER NOT NULL DEFAULT 0,
  mouse_click_count INTEGER NOT NULL DEFAULT 0,
  keyboard_press_count INTEGER NOT NULL DEFAULT 0,
  report_generated INTEGER NOT NULL DEFAULT 0,
  report_score INTEGER,
  report_day_type TEXT,
  first_launch_minute_of_day INTEGER,
  active_00_05_seconds INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (user_id, date_key)
);

CREATE INDEX idx_achievement_daily_rollups_user_active
  ON achievement_daily_rollups(user_id, active_seconds);
```

### 3.5 `achievement_user_unlocks`

```sql
CREATE TABLE achievement_user_unlocks (
  unlock_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  achievement_id TEXT NOT NULL,
  unlocked_at INTEGER NOT NULL,
  points_awarded INTEGER NOT NULL,
  grant_type TEXT NOT NULL DEFAULT 'auto',
  source_event_id TEXT,
  rule_snapshot_json TEXT NOT NULL,
  progress_snapshot_json TEXT NOT NULL,
  revoked_at INTEGER,
  revoke_reason TEXT,
  UNIQUE(user_id, achievement_id),
  FOREIGN KEY (achievement_id) REFERENCES achievement_definitions(id)
);

CREATE INDEX idx_achievement_unlocks_user_time
  ON achievement_user_unlocks(user_id, unlocked_at);
```

`grant_type` 可选值：

| grant_type | 含义 |
| --- | --- |
| `auto` | 正常自动解锁。 |
| `admin_repair` | 管理员数据修复补发。 |
| `migration` | 版本迁移时根据历史数据自动补齐。 |

### 3.6 `achievement_notification_queue`

```sql
CREATE TABLE achievement_notification_queue (
  queue_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  achievement_id TEXT NOT NULL,
  unlock_id TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'pending',
  created_at INTEGER NOT NULL,
  shown_at INTEGER,
  dismissed_at INTEGER,
  priority INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (unlock_id) REFERENCES achievement_user_unlocks(unlock_id)
);

CREATE INDEX idx_achievement_notification_pending
  ON achievement_notification_queue(user_id, state, priority, created_at);
```

### 3.7 `achievement_admin_grants`

```sql
CREATE TABLE achievement_admin_grants (
  repair_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  achievement_id TEXT NOT NULL,
  operator_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  reason TEXT NOT NULL,
  evidence_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
```

管理员补发约束：

- `operation` 仅允许 `grant`、`revoke_admin_grant`。
- `reason` 长度不少于 12 个字符。
- `evidence_json` 必须包含原始计数、目标计数、规则版本和工单号或本地修复任务号。
- 管理员补发不得绕过 `achievement_user_unlocks` 唯一约束。

## ④ 全量 100 + 成就明细清单

本系统首批定义 132 个成就，覆盖原 120 个基础成就，并追加 12 个日报工况卡等级与工况职级成就。

分类键：

| 分类 | category_key |
| --- | --- |
| 日常使用类 | `daily_use` |
| 任务效率类 | `task_efficiency` |
| 长期打卡类 | `long_streak` |
| 功能探索类 | `feature_exploration` |
| 数据里程碑类 | `data_milestone` |
| 工坊养成类 | `workshop_growth` |
| 硬件健康类 | `hardware_health` |
| 社交协作类 | `social_collaboration` |
| 隐藏彩蛋类 | `hidden_easter` |

### 4.1 入门（简单）

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A001 | 日常使用类 | 第一次唤醒 CoCat | 5 | 否 | `cwp_badge_daily_first_launch_entry` | `app.launch.count >= 1`。 |
| A002 | 日常使用类 | 30 分钟陪伴 | 5 | 否 | `cwp_badge_daily_30m_companion_entry` | `lifetime.total_online_seconds >= 1800`。 |
| A003 | 日常使用类 | 第一份工况报告 | 5 | 否 | `cwp_badge_daily_first_report_entry` | `worklog.daily_generated.count >= 1`。 |
| A004 | 功能探索类 | 看过控制台 | 5 | 否 | `cwp_badge_explore_dashboard_entry` | `page.view.count(pageKey='dashboard') >= 1`。 |
| A005 | 功能探索类 | 看过工坊 | 5 | 否 | `cwp_badge_explore_workshop_entry` | `page.view.count(pageKey='workshop') >= 1`。 |
| A006 | 功能探索类 | 看过设备清单 | 5 | 否 | `cwp_badge_explore_devices_entry` | `page.view.count(pageKey='devices') >= 1`。 |
| A007 | 功能探索类 | 保存第一项设置 | 5 | 否 | `cwp_badge_explore_first_setting_entry` | `settings.update.count >= 1`。 |
| A008 | 功能探索类 | 开启悬浮监控条 | 5 | 否 | `cwp_badge_explore_monitor_bar_entry` | `monitor_bar.open.count >= 1`。 |
| A009 | 功能探索类 | 点亮任务栏监控 | 5 | 否 | `cwp_badge_explore_taskbar_monitor_entry` | `settings.update.count(changedKey='showMonitorDataInTaskbar', value=true) >= 1`。 |
| A010 | 功能探索类 | 打开 CoCat 面板 | 5 | 否 | `cwp_badge_explore_pet_panel_entry` | `pet.panel.open.count >= 1`。 |
| A011 | 日常使用类 | 第一次摸摸 CoCat | 5 | 否 | `cwp_badge_daily_first_pet_entry` | `pet.click.count >= 1`。 |
| A012 | 日常使用类 | 搬动小伙伴 | 5 | 否 | `cwp_badge_daily_first_drag_entry` | `pet.drag_end.count >= 1`。 |
| A013 | 工坊养成类 | 第一颗模块螺丝 | 5 | 否 | `cwp_badge_workshop_first_module_entry` | `workshop.module_upgrade.count >= 1`。 |
| A014 | 工坊养成类 | 工坊第一次升级 | 5 | 否 | `cwp_badge_workshop_first_level_entry` | `workshop.level_up.count >= 1`。 |
| A015 | 数据里程碑类 | 100 零件入库 | 5 | 否 | `cwp_badge_data_parts_100_entry` | `lifetime.parts_earned >= 100`。 |
| A016 | 数据里程碑类 | 10 灵感入库 | 5 | 否 | `cwp_badge_data_insight_10_entry` | `lifetime.insight_earned >= 10`。 |
| A017 | 任务效率类 | 键盘热身 | 5 | 否 | `cwp_badge_task_keys_100_entry` | `lifetime.keyboard_press_count >= 100`。 |
| A018 | 任务效率类 | 鼠标热身 | 5 | 否 | `cwp_badge_task_clicks_50_entry` | `lifetime.mouse_click_count >= 50`。 |
| A019 | 社交协作类 | 第一张报告卡 | 5 | 否 | `cwp_badge_social_first_report_export_entry` | `share.report_card.export.count >= 1`。 |
| A020 | 功能探索类 | 打开成就图鉴 | 5 | 否 | `cwp_badge_explore_gallery_entry` | `achievement.gallery.view.count >= 1`。 |

### 4.2 进阶（普通）

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A021 | 日常使用类 | 4 小时陪伴 | 10 | 否 | `cwp_badge_daily_4h_companion_normal` | `lifetime.total_online_seconds >= 14400`。 |
| A022 | 长期打卡类 | 三日有迹 | 10 | 否 | `cwp_badge_streak_3_active_days_normal` | `calendar.days(active_seconds >= 1800) >= 3`。 |
| A023 | 长期打卡类 | 连续三天开工 | 10 | 否 | `cwp_badge_streak_3_consecutive_normal` | `calendar.consecutive_days(active_seconds >= 1800) >= 3`。 |
| A024 | 日常使用类 | 三份工况报告 | 10 | 否 | `cwp_badge_daily_3_reports_normal` | `worklog.daily_generated.count >= 3`。 |
| A025 | 任务效率类 | 三次稳定推进 | 10 | 否 | `cwp_badge_task_score_60_x3_normal` | `calendar.days(report_score >= 60) >= 3`。 |
| A026 | 功能探索类 | 主界面巡礼 | 10 | 否 | `cwp_badge_explore_all_pages_normal` | `distinct_count(page.view.pageKey in ['dashboard','workshop','devices','worklog','settings','about','achievements']) >= 7`。 |
| A027 | 日常使用类 | 30 次抚摸 | 10 | 否 | `cwp_badge_daily_pet_30_normal` | `pet.click.count >= 30`。 |
| A028 | 日常使用类 | 面板常客 | 10 | 否 | `cwp_badge_daily_panel_20_normal` | `pet.panel.open.count >= 20`。 |
| A029 | 日常使用类 | 桌面搬运练习 | 10 | 否 | `cwp_badge_daily_drag_10_normal` | `pet.drag_end.count >= 10`。 |
| A030 | 功能探索类 | 三色试验 | 10 | 否 | `cwp_badge_explore_three_themes_normal` | `distinct_count(settings.update.themeName) >= 3`。 |
| A031 | 功能探索类 | 自定义监控项 | 10 | 否 | `cwp_badge_explore_metric_custom_normal` | `settings.update.count(changedKey='visibleMonitorMetrics') >= 1` 且当前可见指标数 `>= 3`。 |
| A032 | 工坊养成类 | 工坊 3 级 | 10 | 否 | `cwp_badge_workshop_level_3_normal` | `workshop.level >= 3`。 |
| A033 | 工坊养成类 | 单模块 3 级 | 10 | 否 | `cwp_badge_workshop_any_module_3_normal` | `max(workshop.module_level.parts, workshop.module_level.process) >= 3`。 |
| A034 | 工坊养成类 | 三类模块动过手 | 10 | 否 | `cwp_badge_workshop_three_modules_normal` | `distinct_count(workshop.module_upgrade.moduleKey) >= 3`。 |
| A035 | 数据里程碑类 | 1000 零件入库 | 10 | 否 | `cwp_badge_data_parts_1000_normal` | `lifetime.parts_earned >= 1000`。 |
| A036 | 数据里程碑类 | 100 灵感入库 | 10 | 否 | `cwp_badge_data_insight_100_normal` | `lifetime.insight_earned >= 100`。 |
| A037 | 任务效率类 | 1 小时高负载 | 10 | 否 | `cwp_badge_task_high_load_1h_normal` | `lifetime.high_load_seconds >= 3600`。 |
| A038 | 数据里程碑类 | 10 GiB 数据流 | 10 | 否 | `cwp_badge_data_io_10gib_normal` | `lifetime.disk_bytes_total + lifetime.network_bytes_total >= 10737418240`。 |
| A039 | 任务效率类 | 1500 次输入 | 10 | 否 | `cwp_badge_task_input_1500_normal` | `lifetime.keyboard_press_count + lifetime.mouse_click_count >= 1500`。 |
| A040 | 社交协作类 | 第一张徽章卡 | 10 | 否 | `cwp_badge_social_first_badge_export_normal` | `share.achievement_card.export.count >= 1`。 |
| A121 | 任务效率类 | 第一张 B 级工况卡 | 10 | 否 | `cwp_badge_worklog_rarity_b_normal` | `worklog.rarity.max_rank >= 2`。 |

### 4.3 熟练（中等）

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A041 | 日常使用类 | 24 小时陪伴 | 20 | 否 | `cwp_badge_daily_24h_companion_skilled` | `lifetime.total_online_seconds >= 86400`。 |
| A042 | 长期打卡类 | 14 个活跃日 | 20 | 否 | `cwp_badge_streak_14_active_days_skilled` | `calendar.days(active_seconds >= 1800) >= 14`。 |
| A043 | 长期打卡类 | 连续七天开工 | 20 | 否 | `cwp_badge_streak_7_consecutive_skilled` | `calendar.consecutive_days(active_seconds >= 3600) >= 7`。 |
| A044 | 日常使用类 | 14 份工况报告 | 20 | 否 | `cwp_badge_daily_14_reports_skilled` | `worklog.daily_generated.count >= 14`。 |
| A045 | 任务效率类 | 十次优良工况 | 20 | 否 | `cwp_badge_task_score_70_x10_skilled` | `calendar.days(report_score >= 70) >= 10`。 |
| A046 | 任务效率类 | 四种工作日类型 | 20 | 否 | `cwp_badge_task_four_day_types_skilled` | `distinct_count(worklog.daily_generated.dayType where dayType != 'unknown') >= 4`。 |
| A047 | 工坊养成类 | 工坊 10 级 | 20 | 否 | `cwp_badge_workshop_level_10_skilled` | `workshop.level >= 10`。 |
| A048 | 工坊养成类 | 六模块零件 5 级 | 20 | 否 | `cwp_badge_workshop_all_parts_5_skilled` | 6 个模块的 `parts` 等级全部 `>= 5`。 |
| A049 | 工坊养成类 | 六模块工艺 5 级 | 20 | 否 | `cwp_badge_workshop_all_process_5_skilled` | 6 个模块的 `process` 等级全部 `>= 5`。 |
| A050 | 工坊养成类 | 单轨 20 级 | 20 | 否 | `cwp_badge_workshop_any_track_20_skilled` | 任意模块任一升级轨等级 `>= 20`。 |
| A051 | 数据里程碑类 | 10000 零件入库 | 20 | 否 | `cwp_badge_data_parts_10000_skilled` | `lifetime.parts_earned >= 10000`。 |
| A052 | 数据里程碑类 | 1000 灵感入库 | 20 | 否 | `cwp_badge_data_insight_1000_skilled` | `lifetime.insight_earned >= 1000`。 |
| A053 | 任务效率类 | CPU 推进 10 小时 | 20 | 否 | `cwp_badge_task_cpu_50_10h_skilled` | `lifetime.cpu_over_50_seconds >= 36000`。 |
| A054 | 任务效率类 | 内存仓库 5 小时 | 20 | 否 | `cwp_badge_task_ram_70_5h_skilled` | `lifetime.memory_over_70_seconds >= 18000`。 |
| A055 | 任务效率类 | GPU 点亮 3 小时 | 20 | 否 | `cwp_badge_task_gpu_70_3h_skilled` | `lifetime.gpu_over_70_seconds >= 10800`。 |
| A056 | 数据里程碑类 | 100 GiB 本地流转 | 20 | 否 | `cwp_badge_data_disk_100gib_skilled` | `lifetime.disk_bytes_total >= 107374182400`。 |
| A057 | 数据里程碑类 | 50 GiB 网络流转 | 20 | 否 | `cwp_badge_data_network_50gib_skilled` | `lifetime.network_bytes_total >= 53687091200`。 |
| A058 | 日常使用类 | 500 次 CoCat 互动 | 20 | 否 | `cwp_badge_daily_pet_500_skilled` | `pet.click.count + pet.panel.open.count + pet.drag_end.count >= 500`。 |
| A059 | 功能探索类 | 设置调校师 | 20 | 否 | `cwp_badge_explore_settings_20_skilled` | `settings.update.count >= 20` 且 `distinct_count(settings.update.changedKey) >= 5`。 |
| A060 | 社交协作类 | 五张分享卡 | 20 | 否 | `cwp_badge_social_exports_5_skilled` | `share.report_card.export.count + share.achievement_card.export.count + share.profile_snapshot.export.count >= 5`。 |
| A122 | 任务效率类 | 第一张 A 级工况卡 | 20 | 否 | `cwp_badge_worklog_rarity_a_skilled` | `worklog.rarity.max_rank >= 3`。 |
| A127 | 任务效率类 | 高压修复师登阶 | 20 | 否 | `cwp_badge_worklog_title_pressure_lv2_skilled` | `worklog.title_level.pressure >= 2`。 |
| A129 | 任务效率类 | 任意职级三级 | 20 | 否 | `cwp_badge_worklog_title_any_lv3_skilled` | `worklog.title_level.max >= 3`。 |

### 4.4 精英（困难）

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A061 | 日常使用类 | 7 天累计陪伴 | 35 | 否 | `cwp_badge_daily_7d_companion_elite` | `lifetime.total_online_seconds >= 604800`。 |
| A062 | 长期打卡类 | 60 个活跃日 | 35 | 否 | `cwp_badge_streak_60_active_days_elite` | `calendar.days(active_seconds >= 3600) >= 60`。 |
| A063 | 长期打卡类 | 连续 30 天开工 | 35 | 否 | `cwp_badge_streak_30_consecutive_elite` | `calendar.consecutive_days(active_seconds >= 3600) >= 30`。 |
| A064 | 日常使用类 | 60 份工况报告 | 35 | 否 | `cwp_badge_daily_60_reports_elite` | `worklog.daily_generated.count >= 60`。 |
| A065 | 任务效率类 | 三十次高质量工况 | 35 | 否 | `cwp_badge_task_score_75_x30_elite` | `calendar.days(report_score >= 75) >= 30`。 |
| A066 | 任务效率类 | 15 个深度专注日 | 35 | 否 | `cwp_badge_task_deep_focus_15_elite` | `calendar.days(report_day_type = 'deepFocus') >= 15`。 |
| A067 | 任务效率类 | 15 个构建爆发日 | 35 | 否 | `cwp_badge_task_build_burst_15_elite` | `calendar.days(report_day_type = 'buildBurst') >= 15`。 |
| A068 | 工坊养成类 | 工坊 30 级 | 35 | 否 | `cwp_badge_workshop_level_30_elite` | `workshop.level >= 30`。 |
| A069 | 工坊养成类 | 六模块零件 20 级 | 35 | 否 | `cwp_badge_workshop_all_parts_20_elite` | 6 个模块的 `parts` 等级全部 `>= 20`。 |
| A070 | 工坊养成类 | 六模块工艺 20 级 | 35 | 否 | `cwp_badge_workshop_all_process_20_elite` | 6 个模块的 `process` 等级全部 `>= 20`。 |
| A071 | 工坊养成类 | 单轨 50 级 | 35 | 否 | `cwp_badge_workshop_any_track_50_elite` | 任意模块任一升级轨等级 `>= 50`。 |
| A072 | 数据里程碑类 | 100000 零件入库 | 35 | 否 | `cwp_badge_data_parts_100000_elite` | `lifetime.parts_earned >= 100000`。 |
| A073 | 数据里程碑类 | 10000 灵感入库 | 35 | 否 | `cwp_badge_data_insight_10000_elite` | `lifetime.insight_earned >= 10000`。 |
| A074 | 硬件健康类 | 高压但可控 | 35 | 否 | `cwp_badge_health_controlled_pressure_elite` | `lifetime.high_load_seconds >= 360000` 且 `lifetime.thermal_warning_seconds <= 72000`。 |
| A075 | 硬件健康类 | 30 个凉爽活跃日 | 35 | 否 | `cwp_badge_health_30_cool_days_elite` | `calendar.days(active_seconds >= 3600 AND thermal_warning_seconds = 0) >= 30`。 |
| A076 | 功能探索类 | 日报阅读 100 次 | 35 | 否 | `cwp_badge_explore_report_views_100_elite` | `page.view.count(pageKey='worklog') >= 100`。 |
| A077 | 社交协作类 | 二十张协作卡 | 35 | 否 | `cwp_badge_social_exports_20_elite` | `share.report_card.export.count + share.achievement_card.export.count + share.profile_snapshot.export.count >= 20`。 |
| A078 | 社交协作类 | 五次外部快照导入 | 35 | 否 | `cwp_badge_social_imports_5_elite` | `share.profile_snapshot.import.count >= 5`。 |
| A079 | 隐藏彩蛋类 | 夜间守望 | 35 | 是 | `cwp_badge_hidden_night_watch_elite` | `calendar.days(active_00_05_seconds >= 1800) >= 12`。 |
| A080 | 隐藏彩蛋类 | 快速三连 | 35 | 是 | `cwp_badge_hidden_triple_click_elite` | `pet.click_burst.count(clicks >= 3, windowMs <= 2000) >= 10`。 |
| A123 | 任务效率类 | 第一张 S 级工况卡 | 35 | 否 | `cwp_badge_worklog_rarity_s_elite` | `worklog.rarity.max_rank >= 4`。 |
| A125 | 任务效率类 | 七张 A 级以上工况卡 | 35 | 否 | `cwp_badge_worklog_rarity_a_plus_7_elite` | `calendar.days(rarity_rank >= 3) >= 7`。 |
| A128 | 任务效率类 | 高压修复师精进 | 35 | 否 | `cwp_badge_worklog_title_pressure_lv3_elite` | `worklog.title_level.pressure >= 3`。 |

### 4.5 史诗

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A081 | 日常使用类 | 30 天累计陪伴 | 60 | 否 | `cwp_badge_daily_30d_companion_epic` | `lifetime.total_online_seconds >= 2592000`。 |
| A082 | 长期打卡类 | 180 个活跃日 | 60 | 否 | `cwp_badge_streak_180_active_days_epic` | `calendar.days(active_seconds >= 3600) >= 180`。 |
| A083 | 长期打卡类 | 连续 90 天开工 | 60 | 否 | `cwp_badge_streak_90_consecutive_epic` | `calendar.consecutive_days(active_seconds >= 3600) >= 90`。 |
| A084 | 日常使用类 | 180 份工况报告 | 60 | 否 | `cwp_badge_daily_180_reports_epic` | `worklog.daily_generated.count >= 180`。 |
| A085 | 任务效率类 | 百次优秀工况 | 60 | 否 | `cwp_badge_task_score_80_x100_epic` | `calendar.days(report_score >= 80) >= 100`。 |
| A086 | 任务效率类 | 50 个深度专注日 | 60 | 否 | `cwp_badge_task_deep_focus_50_epic` | `calendar.days(report_day_type = 'deepFocus') >= 50`。 |
| A087 | 任务效率类 | 25 个高压抢修日 | 60 | 否 | `cwp_badge_task_pressure_repair_25_epic` | `calendar.days(report_day_type = 'pressureRepair' AND high_load_seconds >= 7200) >= 25`。 |
| A088 | 工坊养成类 | 工坊 60 级 | 60 | 否 | `cwp_badge_workshop_level_60_epic` | `workshop.level >= 60`。 |
| A089 | 工坊养成类 | 六模块零件 50 级 | 60 | 否 | `cwp_badge_workshop_all_parts_50_epic` | 6 个模块的 `parts` 等级全部 `>= 50`。 |
| A090 | 工坊养成类 | 六模块工艺 50 级 | 60 | 否 | `cwp_badge_workshop_all_process_50_epic` | 6 个模块的 `process` 等级全部 `>= 50`。 |
| A091 | 工坊养成类 | 第一条满级轨道 | 60 | 否 | `cwp_badge_workshop_first_track_100_epic` | 任意模块任一升级轨等级 `>= 100`。 |
| A092 | 数据里程碑类 | 百万零件库 | 60 | 否 | `cwp_badge_data_parts_1m_epic` | `lifetime.parts_earned >= 1000000`。 |
| A093 | 数据里程碑类 | 十万灵感库 | 60 | 否 | `cwp_badge_data_insight_100k_epic` | `lifetime.insight_earned >= 100000`。 |
| A094 | 数据里程碑类 | 5 TiB 数据河流 | 60 | 否 | `cwp_badge_data_io_5tib_epic` | `lifetime.disk_bytes_total + lifetime.network_bytes_total >= 5497558138880`。 |
| A095 | 数据里程碑类 | 1 TiB 网络流转 | 60 | 否 | `cwp_badge_data_network_1tib_epic` | `lifetime.network_bytes_total >= 1099511627776`。 |
| A096 | 任务效率类 | 百万次输入 | 60 | 否 | `cwp_badge_task_input_1m_epic` | `lifetime.keyboard_press_count + lifetime.mouse_click_count >= 1000000`。 |
| A097 | 硬件健康类 | 60 个高负载凉爽日 | 60 | 否 | `cwp_badge_health_60_cool_pressure_days_epic` | `calendar.days(high_load_seconds >= 1800 AND thermal_warning_seconds = 0) >= 60`。 |
| A098 | 数据里程碑类 | 解锁 80 项成就 | 60 | 否 | `cwp_badge_data_achievements_80_epic` | `achievement.unlocked.count >= 80`。 |
| A099 | 数据里程碑类 | 九类图鉴都有章 | 60 | 否 | `cwp_badge_data_all_categories_5_epic` | 9 个分类中每个分类已解锁成就数 `>= 5`。 |
| A100 | 隐藏彩蛋类 | 低功耗守护者 | 60 | 是 | `cwp_badge_hidden_low_power_guard_epic` | `calendar.days(low_power_mode_enabled_seconds >= 3600) >= 30` 且 `lifetime.low_power_mode_enabled_seconds >= 2592000`。 |
| A124 | 任务效率类 | 第一张 SS 级工况卡 | 60 | 否 | `cwp_badge_worklog_rarity_ss_epic` | `worklog.rarity.max_rank >= 5`。 |
| A126 | 任务效率类 | 三十张 S 级以上工况卡 | 60 | 否 | `cwp_badge_worklog_rarity_s_plus_30_epic` | `calendar.days(rarity_rank >= 4) >= 30`。 |
| A130 | 任务效率类 | 任意职级满阶 | 60 | 否 | `cwp_badge_worklog_title_any_lv5_epic` | `worklog.title_level.max >= 5`。 |
| A131 | 任务效率类 | 七类职级三级 | 60 | 否 | `cwp_badge_worklog_title_all_lv3_epic` | 7 个 `title_family` 每类等级全部 `>= 3`。 |

### 4.6 传说

| ID | 分类 | 成就名称 | 点数 | 隐藏 | 徽章名称 | 自动解锁条件 |
| --- | --- | --- | ---: | --- | --- | --- |
| A101 | 日常使用类 | 365 天累计陪伴 | 100 | 否 | `cwp_badge_daily_365d_companion_legendary` | `lifetime.total_online_seconds >= 31536000`。 |
| A102 | 长期打卡类 | 365 个活跃日 | 100 | 否 | `cwp_badge_streak_365_active_days_legendary` | `calendar.days(active_seconds >= 3600) >= 365`。 |
| A103 | 长期打卡类 | 连续 180 天开工 | 100 | 否 | `cwp_badge_streak_180_consecutive_legendary` | `calendar.consecutive_days(active_seconds >= 3600) >= 180`。 |
| A104 | 日常使用类 | 365 份工况报告 | 100 | 否 | `cwp_badge_daily_365_reports_legendary` | `worklog.daily_generated.count >= 365`。 |
| A105 | 任务效率类 | 180 次卓越工况 | 100 | 否 | `cwp_badge_task_score_85_x180_legendary` | `calendar.days(report_score >= 85) >= 180`。 |
| A106 | 任务效率类 | 七类工作日全熟练 | 100 | 否 | `cwp_badge_task_all_day_types_30_legendary` | 7 个 `report_day_type` 每类天数全部 `>= 30`，不含 `unknown`。 |
| A107 | 工坊养成类 | 工坊 100 级 | 100 | 否 | `cwp_badge_workshop_level_100_legendary` | `workshop.level >= 100`。 |
| A108 | 工坊养成类 | 十二条模块轨道满级 | 100 | 否 | `cwp_badge_workshop_all_tracks_100_legendary` | 6 个模块的 `parts` 与 `process` 共 12 条轨道全部 `>= 100`。 |
| A109 | 数据里程碑类 | 千万零件库 | 100 | 否 | `cwp_badge_data_parts_10m_legendary` | `lifetime.parts_earned >= 10000000`。 |
| A110 | 数据里程碑类 | 百万灵感库 | 100 | 否 | `cwp_badge_data_insight_1m_legendary` | `lifetime.insight_earned >= 1000000`。 |
| A111 | 数据里程碑类 | 50 TiB 数据星河 | 100 | 否 | `cwp_badge_data_io_50tib_legendary` | `lifetime.disk_bytes_total + lifetime.network_bytes_total >= 54975581388800`。 |
| A112 | 任务效率类 | 千万次输入 | 100 | 否 | `cwp_badge_task_input_10m_legendary` | `lifetime.keyboard_press_count + lifetime.mouse_click_count >= 10000000`。 |
| A113 | 硬件健康类 | 全年无存档修复 | 100 | 否 | `cwp_badge_health_no_corruption_365_legendary` | `calendar.days(active_seconds >= 3600) >= 365` 且 `storage.corruption_rebuilt.count = 0`。 |
| A114 | 数据里程碑类 | 百枚可见徽章 | 100 | 否 | `cwp_badge_data_visible_100_legendary` | `non_hidden_achievement.unlocked.count >= 100`。 |
| A115 | 隐藏彩蛋类 | 十二个月都有工况 | 100 | 是 | `cwp_badge_hidden_12_months_reports_legendary` | `calendar.months(report_generated_days >= 20) >= 12`。 |
| A116 | 隐藏彩蛋类 | CoCat 全动画见证 | 100 | 是 | `cwp_badge_hidden_all_animations_legendary` | 18 个 CoCat 动画状态的 `cocat.animation_seen.count(animationState)` 全部 `>= 10`。 |
| A117 | 隐藏彩蛋类 | 午夜维护长征 | 100 | 是 | `cwp_badge_hidden_midnight_maintenance_legendary` | `calendar.days(active_00_05_seconds >= 1800 AND high_load_seconds >= 1800) >= 30`。 |
| A118 | 隐藏彩蛋类 | 404 修复师 | 100 | 是 | `cwp_badge_hidden_error_glitch_404_legendary` | `cocat.animation_seen.count(animationState='errorGlitch') >= 404`。 |
| A119 | 隐藏彩蛋类 | 六档难度全频段 | 100 | 是 | `cwp_badge_hidden_all_difficulties_legendary` | 6 个难度中每个难度已解锁成就数全部 `>= 10`。 |
| A120 | 隐藏彩蛋类 | 隐藏星图完成 | 100 | 是 | `cwp_badge_hidden_constellation_complete_legendary` | `hidden_achievement.unlocked.count(excludeSelf=true) >= 8`。 |
| A132 | 任务效率类 | 七类职级满阶 | 100 | 否 | `cwp_badge_worklog_title_all_lv5_legendary` | 7 个 `title_family` 每类等级全部 `>= 5`。 |

## ⑤ 徽章绑定规则与前端展示规范

### 5.1 徽章命名规则

徽章资源占位符使用 `badge_key`，必须满足：

```text
cwp_badge_{category_short}_{slug}_{difficulty_key}
```

要求：

- 全小写 snake_case。
- 只允许 `a-z`、`0-9`、`_`。
- 长度不超过 80 个字符。
- 发布后不得修改；若图片换版，只替换资源文件，不改 `badge_key`。
- 图片资源路径约定为 `src/assets/achievements/{badge_key}.webp`。
- 未提供图片时前端使用统一占位图 `src/assets/achievements/cwp_badge_placeholder.webp`。

分类短名：

| category_key | category_short |
| --- | --- |
| `daily_use` | `daily` |
| `task_efficiency` | `task` |
| `long_streak` | `streak` |
| `feature_exploration` | `explore` |
| `data_milestone` | `data` |
| `workshop_growth` | `workshop` |
| `hardware_health` | `health` |
| `social_collaboration` | `social` |
| `hidden_easter` | `hidden` |

### 5.2 图鉴展示

成就图鉴页面必须支持：

- 总点数。
- 已解锁数量 / 可见成就总数。
- 隐藏成就已解锁数量 / 隐藏成就总数。
- 难度筛选。
- 分类筛选。
- 已解锁、未解锁筛选。
- 按解锁时间倒序、难度权重倒序、分类顺序排序。

未解锁可见成就展示：

- 显示标题、分类、难度、点数、徽章灰度占位图、明确进度。
- 进度格式必须可量化，例如 `3 / 7 天`、`12,500 / 100,000 零件`。

未解锁隐藏成就展示：

- 不展示标题、描述、触发条件和徽章名称。
- 只在汇总区显示 `隐藏成就：已解锁 X / 总数 Y`。
- 分类筛选中不显示隐藏成就锁定位。

已解锁隐藏成就展示：

- 正常进入图鉴。
- 详情页标记 `隐藏成就`。
- 展示解锁时间和触发快照。

### 5.3 个人主页陈列

个人主页默认展示：

- 最近解锁 3 个成就。
- 最高难度 3 个成就，难度相同按解锁时间倒序。
- 用户手动置顶 6 个成就。
- 总点数和分难度计数。

置顶规则：

- 最多 6 个。
- 隐藏成就解锁后允许置顶。
- 被撤销的错误补发成就自动从置顶区移除。

## ⑥ 成就解锁推送、弹窗、个人主页陈列规则

### 6.1 推送策略

解锁后 Rust 后端发出：

```text
achievement:unlocked
cocat:interaction-state = achievementPop
```

`achievement:unlocked` payload：

```ts
interface AchievementUnlockedEvent {
  unlockId: string;
  achievementId: string;
  title: string;
  difficultyKey: AchievementDifficulty;
  categoryKey: AchievementCategory;
  points: number;
  badgeKey: string;
  unlockedAt: number;
  isHidden: boolean;
}
```

### 6.2 弹窗队列

规则：

- 同一秒内解锁多个成就时，按难度权重倒序展示；同难度按 `display_order`。
- 弹窗一次只展示 1 个，展示时长 4500 ms。
- 一次批量最多连续展示 3 个，剩余进入通知中心。
- 传说和史诗成就不被折叠，必须进入弹窗队列。
- 用户处于全屏或低功耗模式时，不弹大弹窗，只记录红点和通知中心。

### 6.3 声音和动画

- 如果 `settings.enableSound = true`，播放成就音效 `achievement`.
- 无论是否开启声音，都触发 CoCat `achievementPop` 一次。
- 如果当前处于 `errorGlitch`、`updateInstalling` 等更高优先级动画，成就动画排队到当前 one-shot 结束后播放。

### 6.4 红点和通知中心

- 图鉴页红点显示未查看解锁数。
- 个人主页成就区显示最新解锁 3 项。
- 已查看条件为用户打开成就详情，或通知弹窗完整展示结束。
- 红点状态存入 `achievement_notification_queue.state`。

## ⑦ 重复达成、失效成就、补发、隐藏成就逻辑

### 7.1 重复达成

- 默认 `repeat_policy = once`。
- 已解锁后再次满足条件不新增 `achievement_user_unlocks`，只更新计数器。
- 阶梯目标必须拆成多个成就，例如 100、1000、10000 零件。
- 前端不得根据事件自行判断解锁，只能展示后端返回结果。

### 7.2 成就失效

成就定义可能因规则错误、版本废弃、分类调整而失效：

- `enabled_to_version` 有值时，新版本不再评估该成就。
- 用户历史解锁默认保留，并继续计入总点数。
- 若成就是错误规则导致误发，必须通过管理员修复撤销，写入 `revoked_at`、`revoke_reason` 和 `achievement_admin_grants`。
- 失效成就在图鉴中标记 `已归档`，不再显示进度。

### 7.3 管理员补发

管理员补发只允许以下场景：

- 迁移时历史 JSON 数据可证明已满足条件，但自动迁移失败。
- 规则引擎 bug 导致用户满足条件但未解锁。
- 数据库损坏后从备份恢复，缺少已解锁记录。

补发流程：

1. 管理员输入 `achievement_id`、`user_id`、原因和证据 JSON。
2. 后端校验成就是启用或归档状态。
3. 后端校验用户未拥有该成就。
4. 写入 `achievement_admin_grants`。
5. 写入 `achievement_user_unlocks`，`grant_type = admin_repair`。
6. 不触发普通解锁弹窗，前端只在通知中心显示 `修复补发`。

### 7.4 隐藏成就

- 隐藏成就定义存在于本地数据库中，但 API 默认不返回未解锁隐藏项。
- 规则引擎照常评估隐藏成就。
- 解锁后向前端推送真实标题、描述和徽章。
- 隐藏成就不能依赖人工主观判断。
- 隐藏成就的触发条件可以较难，但必须可由事件和计数器验证。

## ⑧ 接口设计简要说明

### 8.1 Tauri commands

```ts
type AchievementDifficulty =
  | "entry"
  | "normal"
  | "skilled"
  | "elite"
  | "epic"
  | "legendary";

type AchievementCategory =
  | "daily_use"
  | "task_efficiency"
  | "long_streak"
  | "feature_exploration"
  | "data_milestone"
  | "workshop_growth"
  | "hardware_health"
  | "social_collaboration"
  | "hidden_easter";
```

#### `track_achievement_event`

前端通用埋点接口。

```ts
interface TrackAchievementEventRequest {
  eventName: string;
  occurredAt: number;
  idempotencyKey: string;
  payload: Record<string, unknown>;
}

interface TrackAchievementEventResponse {
  accepted: boolean;
  unlocked: AchievementUnlockedEvent[];
}
```

#### `get_achievement_summary`

```ts
interface AchievementSummary {
  totalPoints: number;
  unlockedCount: number;
  visibleTotalCount: number;
  hiddenUnlockedCount: number;
  hiddenTotalCount: number;
  byDifficulty: Record<AchievementDifficulty, { unlocked: number; total: number }>;
  byCategory: Record<AchievementCategory, { unlocked: number; total: number }>;
  latestUnlocks: AchievementCard[];
  pinnedUnlocks: AchievementCard[];
}
```

#### `list_achievements`

```ts
interface ListAchievementsRequest {
  categoryKey?: AchievementCategory;
  difficultyKey?: AchievementDifficulty;
  unlockedState?: "all" | "unlocked" | "locked";
  includeUnlockedHidden?: boolean;
  sort?: "displayOrder" | "unlockedAtDesc" | "difficultyDesc";
}
```

返回规则：

- 未解锁隐藏成就永远不返回。
- `includeUnlockedHidden` 仅影响已解锁隐藏成就是否进入结果。

#### `get_achievement_detail`

返回单个成就详情、进度、解锁快照。

#### `pin_profile_achievement`

最多置顶 6 个。

#### `admin_repair_grant_achievement`

仅 debug/admin 构建可注册。正式 UI 不暴露普通入口。

### 8.2 前端状态

新增 `src/stores/achievementStore.ts`：

```ts
interface AchievementStore {
  summary: AchievementSummary | null;
  cards: AchievementCard[];
  notificationQueue: AchievementUnlockedEvent[];
  loadSummary: () => Promise<void>;
  loadCards: (request: ListAchievementsRequest) => Promise<void>;
  markNotificationShown: (unlockId: string) => Promise<void>;
  pinAchievement: (achievementId: string) => Promise<void>;
  unpinAchievement: (achievementId: string) => Promise<void>;
}
```

### 8.3 后端内部接口

```rust
pub struct AchievementEventInput {
    pub event_name: String,
    pub occurred_at: i64,
    pub source: String,
    pub idempotency_key: String,
    pub payload_json: serde_json::Value,
}

pub struct AchievementUnlockResult {
    pub achievement_id: String,
    pub unlock_id: String,
    pub points_awarded: i32,
}
```

内部 Rust 服务必须提供：

- `record_event(input) -> Vec<AchievementUnlockResult>`
- `evaluate_by_event_name(user_id, event_name) -> Vec<AchievementUnlockResult>`
- `rebuild_counters_from_events(user_id) -> RebuildReport`
- `migrate_from_legacy_storage() -> MigrationReport`

## 9. 落地实施执行方案

### 9.1 前后端功能拆分

后端任务：

1. 新增 SQLite 初始化、迁移、备份。
2. 新增成就定义 seed 文件和导入逻辑。
3. 新增事件表、计数器表、解锁表、通知队列表。
4. 实现通用事件接收、幂等写入、计数器更新。
5. 实现规则 DSL 解析和评估。
6. 将现有工坊、日报、硬件采样、设置保存、CoCat 命令接入事件流。
7. 实现查询接口和管理员补发接口。
8. 实现 JSON 历史数据迁移补齐。

前端任务：

1. 新增 `achievementApi.ts` 与 `achievementStore.ts`。
2. 新增成就图鉴页面，支持筛选、排序、进度展示。
3. 新增成就详情弹窗。
4. 新增解锁弹窗队列和通知中心红点。
5. 新增个人主页成就陈列区。
6. 在页面路由、设置、分享导出、CoCat 交互处补齐 `track_achievement_event`。
7. 为隐藏成就、归档成就、补发成就做展示差异。

### 9.2 开发优先级排序

| 优先级 | 范围 | 目标 |
| --- | --- | --- |
| P0 | 数据库、定义 seed、规则 DSL | 能加载 132 条成就并完成规则单测。 |
| P1 | 事件接入、计数器、自动解锁 | 工坊、日报、在线时长、页面访问可自动解锁。 |
| P2 | 图鉴、弹窗、CoCat 动画 | 用户可见闭环完成。 |
| P3 | 全量埋点接入 | 设置、宠物、监控条、分享、硬件健康全部接入。 |
| P4 | 隐藏成就、补发、归档 | 高级逻辑与异常处理完成。 |
| P5 | 迁移、灰度、校验、回滚 | 可稳定发布到正式版本。 |

首版建议发布最小闭环：

- 启用 A001 到 A060。
- A061 到 A132 先 seed 入库但 `enabled_from_version` 指向下一小版本或使用 feature flag 关闭评估。
- 确认规则引擎和图鉴稳定后再打开高难度成就评估。

### 9.3 单元测试用例设计

后端测试：

| 测试名 | 断言 |
| --- | --- |
| `seed_contains_132_unique_achievements` | 成就数量等于 132，ID、code、badge_key 唯一。 |
| `difficulty_points_are_valid` | 六档点数分别为 5、10、20、35、60、100。 |
| `hidden_locked_items_are_not_listed` | 未解锁隐藏成就不出现在列表 API。 |
| `event_idempotency_prevents_duplicate_unlocks` | 同一 `idempotency_key` 重放不重复计数、不重复解锁。 |
| `counter_condition_unlocks_entry_achievement` | `app.launch.count >= 1` 自动解锁 A001。 |
| `all_condition_requires_all_nodes` | `all` 节点任一不满足时不解锁。 |
| `distinct_counter_counts_unique_values` | 页面去重访问正确计算 A026。 |
| `consecutive_days_counter_resets_on_gap` | 连续天数中断后重新累计。 |
| `per_bucket_min_checks_all_modules` | 12 条模块轨道全部达标才解锁 A108。 |
| `admin_repair_requires_reason_and_evidence` | 缺少原因或证据时补发失败。 |
| `revoked_unlock_does_not_count_points` | 撤销后总点数扣除对应点数。 |
| `migration_unlock_uses_migration_grant_type` | 历史数据补齐写入 `migration`。 |

前端测试：

| 测试名 | 断言 |
| --- | --- |
| `gallery_filters_by_difficulty` | 难度筛选只显示对应卡片。 |
| `gallery_filters_by_category` | 分类筛选只显示对应分类。 |
| `locked_hidden_is_not_rendered` | 未解锁隐藏成就不渲染 DOM 文本。 |
| `unlock_modal_orders_by_difficulty` | 批量解锁按难度优先级展示。 |
| `profile_shelf_limits_to_six_pins` | 置顶最多 6 个。 |
| `placeholder_badge_is_used_when_asset_missing` | 图片缺失时展示占位图。 |

集成测试：

- 启动应用后触发 `app.launch`，A001 解锁。
- 模拟工坊升级到 3 级，A032 解锁。
- 注入 7 天日聚合，A043 解锁。
- 注入隐藏成就条件，列表 API 在解锁前不返回，解锁后返回。
- 模拟一次批量解锁 5 项，弹窗展示 3 项，剩余进入通知中心。

### 9.4 灰度上线策略

桌面端灰度通过版本、feature flag 和本地迁移保护实现：

1. `achievementSystemEnabled=false` 发布数据库迁移和 seed，不展示入口。
2. 内部测试版开启 A001 到 A020，验证事件量、幂等、弹窗。
3. 小范围灰度开启 A001 到 A060，验证用户存档兼容。
4. 正式开启 A001 到 A060，A061 到 A132 保持定义可见但高难度自然长周期解锁。
5. 隐藏成就评估单独使用 `hiddenAchievementEnabled` 开关。
6. 若未来有云同步，只上传匿名聚合校验项，不上传硬件明细。

### 9.5 线上数据校验规则

每日或每次启动执行本地校验：

| 校验项 | 规则 |
| --- | --- |
| 定义完整性 | `achievement_definitions` 数量 `>= 132`，启用定义 ID 唯一，徽章名唯一。 |
| 解锁唯一性 | `achievement_user_unlocks` 不存在同一 `user_id + achievement_id` 多条未撤销记录。 |
| 点数一致性 | `summary.totalPoints = SUM(points_awarded where revoked_at IS NULL)`。 |
| 隐藏泄露 | 未解锁隐藏成就不得出现在 `list_achievements` 响应。 |
| 事件幂等 | 同一 `user_id + idempotency_key` 只存在 1 条事件。 |
| 计数非负 | 所有累计计数器 `numeric_value >= 0`。 |
| 日聚合边界 | 单日 `active_seconds <= 86400`，`active_00_05_seconds <= 18000`。 |
| 弹窗队列 | `pending` 队列中同一 `unlock_id` 只存在 1 条。 |
| 规则 JSON | 所有启用成就的 `condition_json` 能被解析并通过 schema 校验。 |

发现异常时：

- 轻微异常写入 `achievement_integrity.log`。
- 可自动修复的计数器异常调用 `rebuild_counters_from_events`。
- 涉及解锁记录的异常只提示并要求管理员修复，不自动删除用户成就。

### 9.6 异常回滚方案

回滚开关：

| 开关 | 作用 |
| --- | --- |
| `achievementSystemEnabled` | 关闭所有图鉴入口、事件评估、弹窗。 |
| `achievementPopupEnabled` | 只关闭弹窗，不关闭解锁。 |
| `achievementRuleEngineEnabled` | 只记录事件和计数，不产生新解锁。 |
| `hiddenAchievementEnabled` | 关闭隐藏成就评估。 |

回滚步骤：

1. 关闭 `achievementRuleEngineEnabled`，停止新增解锁。
2. 保留事件写入，用于后续修复和重放。
3. 关闭 `achievementPopupEnabled`，避免错误弹窗继续打扰用户。
4. 若数据库迁移失败，恢复 `{app_data_root}/backups/achievements.sqlite.{timestamp}.bak`。
5. 发布修复版本后运行 `rebuild_counters_from_events`。
6. 对漏发用户使用迁移补齐或管理员补发，不通过前端人工发放。

不得执行的回滚动作：

- 不得删除用户已解锁成就。
- 不得清空事件表。
- 不得用前端本地状态覆盖后端解锁记录。
- 不得把隐藏成就条件公开给未解锁用户。

## 10. 与现有 CoworkPal 模块的接入点

| 现有模块 | 接入方式 | 关联成就 |
| --- | --- | --- |
| `StorageService` | 新增 SQLite 初始化、备份、损坏事件 `storage.corruption_rebuilt`。 | A113 |
| `WorkshopState` | 从 `totalOnlineSeconds`、`todayParts`、`todayInsight`、模块等级产生计数。 | A002、A032、A047、A108 |
| `WorkLogEntry` | 从 active seconds、负载秒数、IO、输入和 day type 生成日聚合。 | A043、A085、A106 |
| `PetWindow` | 记录点击、拖动、连击、动画状态。 | A011、A058、A080、A116 |
| `PetQuickPanelWindow` | 记录面板打开和关闭。 | A010、A028 |
| `SettingsPage` | 记录设置修改、主题、监控指标、低功耗时长。 | A007、A030、A100 |
| `WorkLogPage` | 记录日报查看、导出。 | A019、A076 |
| `window_manager` / commands | 记录监控条、任务栏监控、页面窗口显示。 | A008、A009 |
| `cocat:interaction-state` | 成就解锁时触发 `achievementPop`。 | 所有自动解锁成就 |
