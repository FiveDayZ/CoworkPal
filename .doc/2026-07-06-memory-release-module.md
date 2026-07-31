# 内存释放模块设计

> 对齐 [henrypp/memreduct](https://github.com/henrypp/memreduct) 的内存回收机制，为 CoreCat 新增手动 + 自动两套内存释放能力。

## 决策汇总

| 维度 | 选定方案 |
|------|----------|
| 清理对象 | 系统为主、自身为辅（对齐 memreduct） |
| 提权架构 | 主程序自重启（`ShellExecuteW("runas", current_exe, "--memory-clean-helper ...")`），不内嵌 manifest、无独立 helper 二进制 |
| 手动触发 | 托盘菜单"释放内存" → 主程序 `ShellExecuteW("runas")` 自重启 → UAC 通过后全量清理；拒绝则降级为轻档 |
| 自动触发 | 仅阈值：监测系统已用物理内存，超过阈值（默认 8 GB，可调 1–64）触发**轻档**清理，60 秒冷却 |
| 结果反馈 | 系统通知"释放了 X MB"（复用 `enableNotifications`） |

### 提权架构说明（最终采用方案：主程序自重启）

核心思路：**主可执行文件既是 App 也是提权 worker**。

- `main.rs` 在最开头（单实例锁、Tauri 初始化**之前**）检查 argv，命中 `--memory-clean-helper` 就走 worker 分支：执行全量清理、写结果文件、退出，完全不启动 App。
- 手动释放时，主进程用 `ShellExecuteW("runas", current_exe, "--memory-clean-helper --result-file=<path>")` 重启自己，Windows 弹 UAC；用户接受后 worker 实例以管理员权限跑完全量清理退出，原进程等待并读取结果。
- 普通 `cargo run` / 开机自启**永不**带这个参数，所以永不弹 UAC。

**为何不用独立 helper 二进制**：
1. `embed-resource` 内嵌 `requireAdministrator` manifest 会全局污染 package 内所有 bin target（[cargo #12848](https://github.com/rust-lang/cargo/issues/12848)），无法只针对 helper，会让主程序也变成每次启动弹 UAC。
2. `bundle.resources` / `externalBin` 打包独立二进制会在 build.rs 阶段（编译产物尚未生成时）触发路径校验，导致 `resource path ... doesn't exist` 编译失败。
3. 自重启方案彻底消除打包配置：`current_exe()` 在 dev（`target/<profile>/`）和安装后都永远可靠，零额外二进制。

**IPC 协议**（文件而非 stdout）：`ShellExecuteW` 不给父进程可用的 stdout pipe，所以 worker 把结果 `{"releasedBytes": N}` 写入主进程传入的临时文件（`%TEMP%/coworkpal-memory-release-<pid>.json`）。原进程 `WaitForSingleObject`（30 秒超时）后读文件、删除。

### 监测对象决策

需求字面写"实时监测进程内存占用"，但结合"对齐 memreduct"和 GB 单位阈值，模块监测的是 **系统已用物理内存**（`GlobalMemoryStatusEx.ullTotalPhys - ullAvailPhys`）。理由：
- memreduct 监测的就是系统级 `dwMemoryLoad`
- GB 阈值在系统级语义清晰（"系统用了 8GB 就清理"）
- 用户实际关心的是系统整体内存压力，而非单个进程

## 三档清理策略

| 档位 | 操作 | 提权 | 触发 |
|------|------|------|------|
| **轻档** | ① `MemoryPurgeLowPriorityStandbyList`（清低优先级 Standby）② `EmptyWorkingSet(GetCurrentProcess())`（清自身工作集） | 否 | 自动阈值触发；UAC 被拒时的降级 |
| **全量档** | 轻档全部 + ③ `MemoryPurgeStandbyList`（清全部 Standby）④ `SystemFileCacheInformation`（清系统文件缓存工作集）⑤ `MemoryPurgeModifiedList`（清 Modified List）⑥ 遍历所有进程 `EmptyWorkingSet` | 是 | 手动触发、UAC 通过后 |

自动触发只跑轻档——避免后台触发把用户机器卡住。

## 核心 API 清单

```text
NtSetSystemInformation (ntdll, 动态解析)
  - SystemMemoryListInformation (0x50) + SYSTEM_MEMORY_LIST_COMMAND
      MemoryPurgeLowPriorityStandbyList = 3  (轻档)
      MemoryPurgeStandbyList            = 2  (全量)
      MemoryPurgeModifiedList           = 4  (全量)
  - SystemFileCacheInformation (0x1A) + FILE_CACHE_INFORMATION
      Min/MaxWorkingSet = (SIZE_T)-1         (全量)

EmptyWorkingSet (psapi, 通过 SetProcessWorkingSetSize(-1,-1) 实现)
  - GetCurrentProcess()                     (轻档)
  - 遍历 NtQuerySystemInformation 进程列表   (全量)

GlobalMemoryStatusEx  —— 监测系统内存占用百分比与已用字节数
```

## 组件清单

### 后端 Rust（主进程，普通权限）
- `src-tauri/src/memory_release/mod.rs` — 模块入口、`MemoryReleaseService`（配置 + 冷却时间戳）、`run_light_clean()`、`run_full_clean_via_helper()`、`check_auto_release()`
- `src-tauri/src/memory_release/windows_api.rs` — 轻档 FFI 实现（动态解析 `NtSetSystemInformation`，封装 unsafe）
- `src-tauri/src/memory_release/sysinfo_query.rs` — `GlobalMemoryStatusEx` 查询系统内存占用

### 辅助进程（管理员权限）
- `src-tauri/src/bin/memory_helper.rs` — 独立 `[[bin]]`，manifest=requireAdministrator，参数 `--full-clean`，stdout 输出 `{"released_bytes":N,"downgraded":false}`
- `src-tauri/memory-helper.exe.manifest` — manifest 文件
- `src-tauri/build.rs` — 用 `embed-resource` 嵌入 helper 的 manifest

### 主进程集成点
- `src-tauri/src/tray/mod.rs` — 加 `MENU_RELEASE_MEMORY` 菜单项
- `src-tauri/src/models.rs` — `AppSettings` + `AppSettingsPatch` 加 4 字段
- `src-tauri/src/commands/mod.rs` — 新 command `trigger_memory_release`、`get_memory_status`
- `src-tauri/src/events.rs` — 加 `MEMORY_RELEASE_COMPLETED`
- `src-tauri/src/lib.rs` — `mod memory_release;`、注册 command、启动自动监测循环
- `src-tauri/Cargo.toml` — 加 `[[bin]]`、`windows` features 增补、加 `embed-resource`

### 前端 TS/React
- `src/types/settings.ts` — `AppSettings` 加 4 字段
- `src/services/tauriCommands.ts` — `triggerMemoryRelease()`、`getMemoryStatus()`
- `src/services/events/mainWindowEvents.ts` — 监听 `memory:release-completed`
- `src/pages/settings/SettingsPage.tsx` — 新增"内存释放"卡片

## 新增设置字段（4 个）

```rust
// AppSettings (#[serde(default)])
memory_release_enabled: bool,             // 总开关，默认 true
memory_auto_release_enabled: bool,        // 自动释放开关，默认 false
memory_auto_release_threshold_gb: f64,    // GB 阈值，默认 8.0（范围 1–64）
memory_last_release: Option<LastReleaseRecord>,  // 上次释放记录（时间 + 字节数），不暴露编辑
```

## 稳定性策略

- **UAC 被拒**：helper 启动失败/返回非零 → 降级 `run_light_clean()`，通知标注"（未提权）"
- **helper 崩溃/超时**：主进程 `wait` 设 10 秒超时，超时降级轻档
- **冷却**：自动触发 60 秒冷却（`Instant` 时间戳比对）
- **panic 隔离**：unsafe FFI 在 helper 内（全量）或 `catch_unwind`（轻档），不污染主进程
- **失败容忍**：每个清理 API 单独检查 NTSTATUS，失败跳过不抛错
