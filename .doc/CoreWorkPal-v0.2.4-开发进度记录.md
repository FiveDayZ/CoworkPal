# 代码修改进度记录

> 本文档记录 CoreWorkPal 从 v0.2.2 到 v0.2.4 的全部代码变更，涵盖：架构优化 7 个 PR、3 个新功能（健康提醒 / 专注力仪式 / 节律图谱）、2 次 bug 热修复、2 次版本发布。

---

## 1. 修改时间

- **2026-07-03 08:58** （文档生成时间）
- 变更跨度：v0.2.2 → v0.2.3 → v0.2.4
- 涉及版本发布：`v0.2.2`、`v0.2.3`、`v0.2.4`（均已打 tag 并推送，触发 release CI）

---

## 2. 本次修改模块/文件

### 2.1 架构优化 7 个 PR（v0.2.2）

| PR | 分支 | 涉及文件 |
|---|---|---|
| #1 CSP + 本地字体 | `feat/csp-and-local-fonts` | `src-tauri/tauri.conf.json`、`src/styles/core-ui.css`、`src/assets/fonts/*` (3 个 woff2) |
| #2 已确认 Bug 修复 | `fix/confirmed-bugs` | `src-tauri/src/achievements/runtime.rs`、`src-tauri/src/commands/updater.rs`、`src/services/formatters.ts` |
| #3 spawn_blocking | `perf/spawn-blocking-for-hardware-sample` | `src-tauri/src/app_state.rs`、`src-tauri/src/monitoring/mod.rs`、`src-tauri/src/commands/mod.rs` |
| #4 原子写入 + SMBIOS 缓存 | `fix/atomic-writes-and-smbios-cache` | `src-tauri/src/storage/mod.rs` |
| #5 pet 原子化 + 事件常量 | `refactor/atomic-pet-state-and-event-constants` | `src-tauri/src/{app_state,commands/mod,commands/updater,input_activity,lib,models,monitoring/mod,pet/mod,tray/mod,window_manager/mod}.rs`、**新增** `src-tauri/src/events.rs` |
| #6 前端 ErrorBoundary + 去重 | `refactor/frontend-errorboundary-dedup-deadcode` | **新增** `src/components/ErrorBoundary.tsx`、`src/services/clipboard.ts`、`src/services/monitorMetrics.ts`；删除 3 个死代码文件 |
| #7 UpdateModal 修复 | `refactor/petwindow-hooks-and-update-modal-cleanup` | `src/components/UpdateModal.tsx`、`src/styles/core-ui.css` |

### 2.2 新功能 3 个（v0.2.3）

| 功能 | 涉及文件（新增/修改） |
|---|---|
| #6 健康提醒 | `models.rs`（CatState 新增 Fatigued/NeedsBreak + CatRuntimeState 字段）、`pet/mod.rs`、`input_activity.rs`、`types/pet.ts`、`coreCatStates.ts` |
| #4 专注力仪式 | `models.rs`（FocusSession 系列类型 + CatState 新增 DeepWork/Distracted）、`storage/mod.rs`、`app_state.rs`、`commands/mod.rs`、`events.rs`、`lib.rs`、`monitoring/mod.rs`、`pet/mod.rs`；**新增** `types/focus.ts`、`stores/focusStore.ts`；`tauriCommands.ts`、`petPanelWindowEvents.ts`、`PetQuickPanelWindow.tsx` |
| #1 节律图谱 | `models.rs`（RhythmProfile + build_rhythm_profile）、`commands/mod.rs`、`lib.rs`；**新增** `types/rhythm.ts`；`tauriCommands.ts`、`workLogStore.ts`、`WorkLogPage.tsx` |

### 2.3 Bug 修复与动画接入（v0.2.3 hotfix + v0.2.4）

| 修复 | 涉及文件 |
|---|---|
| tsc 编译失败 | `src/services/catStateRules.ts` |
| Fatigued/NeedsBreak 动画接入 | `src/assets/pets/animation/{Fatigued,NeedsBreak}.{webp,json}`；`animationTypes.ts`、`spriteSheetAssets.ts`、`animationConfig.ts`、`coreCatStates.ts`、`run-corecat-animation-tests.mjs` |
| 专注菜单可见性 | `src-tauri/src/window_manager/mod.rs`、`src/styles/core-ui.css`、`src/pages/dashboard/DashboardPage.tsx`、`src/services/events/mainWindowEvents.ts` |

---

## 3. 新增/优化内容

### 3.1 安全与基线（PR #1）
- **启用 CSP**：`tauri.conf.json` 的 `app.security.csp` 从 `null` 改为显式策略（`default-src 'self'` + 必要来源），关闭 XSS 缓解缺口
- **字体本地化**：将 `@import url('https://fonts.googleapis.com/...')` 替换为本地 `@font-face`（Silkscreen 400/700、VT323），消除每次加载的联网请求，兑现"离线不联网"承诺

### 3.2 后端并发与正确性（PR #3/#4/#5）
- **spawn_blocking**：`HardwareSensorAdapter::sample()`（会 spawn nvidia-smi/powershell 子进程）从 async runtime 移到 `tokio::task::spawn_blocking`，不再阻塞 Tauri runtime 线程；`hardware_adapter` 改为 `Arc<Mutex<...>>`
- **原子文件写入**：`StorageService::write_json` 移除 `remove_file` 步骤，单次 `rename` 覆盖（Windows 上原子），消除崩溃丢文件窗口
- **SMBIOS UUID 缓存**：`query_smbios_uuid`（spawn powershell）通过 `OnceLock` 每实例最多执行一次，避免每次 `load_or_create_settings` 都调用
- **pet 状态原子化**：5 个独立 `RwLock`（cat_state/cat_message/last_cat_state_changed_at/temperature_safe_since/has_emitted_cat_state）合并为单一 `CatRuntimeState` 结构体 + 一个锁，消除读/决策/写竞态
- **事件名常量化**：新增 `src-tauri/src/events.rs`，9 个事件名（`hardware:metrics` 等）从散落字符串改为 `pub const`，所有 `app.emit()` 引用常量

### 3.3 前端工程化（PR #6/#7）
- **ErrorBoundary**：新增 class 组件，包裹 `MainWindow` 的 `<CurrentPage/>`，捕获渲染异常显示兜底 UI（替代白屏崩溃）
- **去重**：`copyTextToClipboard`（原 WorkLog/Achievements 各一份）、`getDisplayedMetrics`（原 MonitorBar/Taskbar 各一份）提取为共享模块
- **删死代码**：`monitorTaskbarText.ts`、`CoreCatLayer.tsx`、`CoreCatVfxLayer.tsx`（无引用的 shim）
- **UpdateModal spinner**：补 `@keyframes spin`（原内联引用未定义的 keyframe，spinner 静默不转）；3 处 `catch (e: any)` → `unknown` + `toErrorMessage` helper

### 3.4 新功能 #6 健康提醒
- CoreCat 连续工作过久（~90min）进入 **Fatigued** 状态，久坐无输入（~50min）进入 **NeedsBreak** 状态
- `CatRuntimeState` 新增 `last_input_at` / `continuous_work_since`，由 `input_activity` flush 维护
- 健康提醒仅在系统无负载/温度告警时触发，不掩盖真实警报

### 3.5 新功能 #4 专注力仪式
- 用户向 CoreCat 交付任务 + 设定专注块（25/50 分钟），期间检测输入活跃度
- **分心检测**：专注会话中输入沉默 >90s 计一次分心（去重），CoreCat 进入 **DeepWork**（专注）或 **Distracted**（分心）状态
- **工坊奖励落地**：完成时按 `focus_quality = 1 - 分心次数×0.15`（下限 0.4）计算质量，给 workshop parts/insight 奖励（修复了原 `reward_corecat_interaction` 只读不改的设计缺陷）
- 持久化：新增 `focus_sessions.json`；3 个命令 `start/complete/abandon_focus_session`

### 3.6 新功能 #1 节律图谱
- 聚合历史 15 分钟时间片到 **24 小时 × 7 天**热力图，识别个人高效时段
- `build_rhythm_profile` 用 chrono Local 推导每个时间片的 hour/weekday，聚合 `active_seconds` + 交叉历史画像分
- WorkLogPage 新增「节律」tab：24 格热力图（peak hours 青色高亮）+ 7 天条形图 + 叙事总结

### 3.7 动画接入（v0.2.4）
- Fatigued/NeedsBreak 从临时复用 sleep/idle 改为**专属动画**
- 接入全链路 5 处：`animationTypes`（union）、`spriteSheetAssets`（STATE_TO_STEM）、`animationConfig`（优先级 66/63）、`coreCatStates`（映射）、`run-corecat-animation-tests.mjs`（mappedStems）

### 3.8 专注菜单可见性（v0.2.4）
- 宠物面板窗口高度 360 → **520px**，`overflow-y: auto`（带细滚动条），解决专注块被裁剪
- Dashboard 新增**专注仪式卡片**（任务输入 + 时长选择 + 倒计时 + 完成/放弃），不再需要找宠物面板
- `mainWindowEvents` 注册 `focus:session-updated`，Dashboard 与面板状态同步

---

## 4. 已修复 Bug

### 4.1 更新器 header 名错误（PR #2）
- **现象**：`updater.rs:170` env-token 分支用 `.header("token", ...)`，应为 `"Authorization"`，导致 ambient-token 下载从未认证
- **修复**：改为 `.header("Authorization", format!("token {}", token))`

### 4.2 std::process::exit 绕过优雅关闭（PR #2）
- **现象**：`install_update` 用 `std::process::exit(0)`，跳过 Tauri 窗口清理/drop
- **修复**：注入 `AppHandle`，改用 `app.exit(0)`

### 4.3 成就条件重复分支（PR #2）
- **现象**：`runtime.rs:927-928` 同一字符串 `contains` 两次（复制粘贴 bug）
- **修复**：删除重复分支

### 4.4 formatBytes 单位数组（PR #2）
- **现象**：`units` 数组带 `/s` 后缀再 strip，输出正确纯属偶然
- **修复**：改为纯字节单位 `["B","KB","MB","GB"]`

### 4.5 更新器静默错误（PR #2）
- **现象**：`file.set_len(0).ok()` 静默忽略截断失败，可能导致断点续传数据损坏；SHA256 校验跳过无日志
- **修复**：截断失败改为返回错误；补 info/warn 日志

### 4.6 catStateRules tsc 编译失败（v0.2.3 hotfix，阻塞 release）
- **现象**：新增 4 个 CatState 变体后，`catStateRules.ts` 的 `messageForCatState` switch 未覆盖新变体也无 default，TS 推断返回 `string | undefined`，赋给 `catMessage: string` 报错。CI `tsc --noEmit` 失败阻断 v0.2.3 release
- **修复**：补全 4 个变体分支 + `default: return ""`，返回类型显式标注 `string`；`catStateSeverity` 同步补全（与后端 severity 一致）
- **根因教训**：本地验证误用 `npx tsc | tail; echo $?`，`$?` 捕获的是 `tail` 退出码而非 tsc 的

### 4.7 专注菜单不可见（v0.2.4）
- **现象**：①面板窗口固定 300×360 + `overflow: hidden`，专注块被物理裁剪；②Dashboard 无专注入口
- **修复**：窗口加高到 520px + 滚动；Dashboard 加专注卡片

### 4.8 动画资源尺寸不一致（v0.2.4）
- **现象**：用户新增的 Fatigued/NeedsBreak 是 5760×5760（帧 720），是其他动画（1280×1280/帧 160）的 20 倍像素量
- **修复**：用 PIL LANCZOS 缩到 1280×1280 + WebP q90 压缩，重写 JSON 坐标；Fatigued 1.15MB→467KB，NeedsBreak 0.95MB→384KB

---

## 5. 待完成&待优化项

### 5.1 已识别但本轮未做的重构
- **`models.rs`（3354 行）拆分**：建议拆为 `models/{settings,workshop,work_log,hardware,assessment}.rs` 子树
- **`core-ui.css`（6600+ 行）拆分**：按 feature 分文件或采用 `@layer`
- **`PetWindow.tsx`（896 行）拆 hooks**：`usePetDrag`/`usePetInteractionQueue`/`usePetLowPower` 等
- **成就条件 AST 落地**：`runtime.rs` 的字符串 `contains` 引擎（~150 行）替换为已有的 `AchievementCondition` AST 结构（当前定义了但未用于评估）

### 5.2 工程化（用户本轮未选）
- **无 lint/format**：建议加 ESLint + Prettier + `eslint-plugin-react-hooks`
- **无前端测试框架**：建议加 Vitest，覆盖 `catStateRules`、workshop 数值、成就触发等纯逻辑
- **无非 release 的 CI**：当前 `.github/workflows/release.yml` 仅在 `v*` tag 触发，普通 push/PR 不跑任何检查
- **文档纳入 git**：`docs/`、`.docs/` 被 gitignore，权威 CoreCat 规范仅本地存在
- **依赖偏旧**：`reqwest 0.11`（0.12 已发布）、`sysinfo 0.30`（0.34+）

### 5.3 新功能的潜在优化
- **专注分心检测**：当前仅基于输入活跃度，可考虑加窗口切换检测（需 `GetForegroundWindow` 轮询，有隐私权衡）
- **节律图谱**：当前只反映"活跃强度"，可结合进程分类区分"开发/娱乐时段"
- **专注会话历史统计**：已完成/放弃的会话已持久化但前端未展示统计页
- **新猫状态美术**：DeepWork/Distracted 仍复用 repairing/idle 动画，待专属美术资源
- **Tauri 内置 updater 迁移**：当前自定义更新器的 SHA256 校验是可选的（仅 sidecar 存在才校验），建议迁移到 Tauri 内置 updater 插件（签名验证，`TAURI_SIGNING_PRIVATE_KEY` 已配置但未被使用）

### 5.4 风险点
- `commands::updater` 的 2 个测试打真实 GitHub API，CI 可能 flaky
- `tauri-apps/tauri-action@v0` 是浮动大版本，建议 pin SHA
- pet 面板加高到 520px 后，低分辨率屏幕可能溢出（`max-height: 100vh` 已兜底）

---

## 6. 补充备注

### 6.1 版本发布记录
| 版本 | tag | 主要内容 |
|---|---|---|
| v0.2.2 | `v0.2.2` | 架构优化 7 个 PR（安全/并发/前端） |
| v0.2.3 | `v0.2.3`（重建过） | 3 个新功能 + tsc hotfix；首次 tag 因 tsc 失败，删除后重建指向修复提交 |
| v0.2.4 | `v0.2.4` | Fatigued/NeedsBreak 动画接入 + 专注菜单可见性修复 |

### 6.2 测试覆盖
- Rust 单元测试：原 54 个 → **64 个**（新增 10：3 健康提醒状态机 + 2 专注状态 + 3 focus_quality 计算 + 2 节律聚合）
- 前端动画测试：`pnpm test:corecat`（`run-corecat-animation-tests.mjs`）验证 sprite sheet 尺寸/坐标一致性，新增 Fatigued/NeedsBreak 后已更新 mappedStems

### 6.3 新增文件清单
**后端**：`src-tauri/src/events.rs`
**前端**：`src/components/ErrorBoundary.tsx`、`src/services/clipboard.ts`、`src/services/monitorMetrics.ts`、`src/stores/focusStore.ts`、`src/types/focus.ts`、`src/types/rhythm.ts`、`src/assets/fonts/*.woff2`（3）、`src/assets/pets/animation/{Fatigued,NeedsBreak}.{webp,json}`（4）

### 6.4 持久化新增
- `focus_sessions.json`：专注会话历史（`FocusSessionBook`）
- `cat_runtime` 状态字段（last_input_at/continuous_work_since/active_focus_session_id/last_distraction_at）为纯内存态，不持久化

### 6.5 开发环境备注
- 包管理器：pnpm 11.6.0（CI 通过 `npx pnpm` 调用）
- 动画压缩工具：Python PIL 12.2.0（项目 `scripts/optimize-animation-pngs.mjs` 仅处理 PNG，WebP 需用 PIL/cwebp）
- 本地无 `gh` CLI，PR 创建通过 GitHub 网页链接手动完成
- release CI 触发条件：`tags: v*`（必须用 `v` 前缀）

### 6.6 验证命令速查
```bash
# Rust
cd src-tauri && cargo check
cd src-tauri && cargo test --lib -- --skip commands::updater  # 跳过联网测试

# 前端（注意：不要接 tail 管道判断退出码）
npx tsc --noEmit; echo $?        # 必须退出码 0
npx vite build; echo $?
node scripts/run-corecat-animation-tests.mjs

# 动画压缩（WebP）
python -c "from PIL import Image; ..."  # LANCZOS 缩放 + 重写 JSON 坐标
```
