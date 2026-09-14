use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    time::{Duration, Instant},
};

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::{
    achievements::{AchievementBook, AchievementDailyRollup, ACHIEVEMENT_BOOK_SCHEMA_VERSION},
    app_state::AppState,
    events::{
        ACHIEVEMENT_PROGRESS_UPDATED, FOCUS_SESSION_UPDATED, NOTES_UPDATED, SETTINGS_UPDATED,
        WORKSHOP_UPDATED,
    },
    models::{
        current_timestamp_ms, AppSettings, FocusSession, FocusSessionBook, FocusSessionStatus,
        HardwareDeviceInventory, HardwareSnapshot, LayoutState, Note, NoteBook, WorkLogBook,
        WorkLogEntry, WorkshopState, WORK_LOG_BOOK_SCHEMA_VERSION,
    },
};

static TOKEN_REQUEST_CHECK_LOCK: Mutex<()> = Mutex::const_new(());
static SYNC_LOCK: Mutex<()> = Mutex::const_new(());

type AutoBackupSchedule = (String, String, String, u64, Instant);

const DEFAULT_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 30;
const MIN_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 5;
const MAX_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 24 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SyncConfig {
    pub server_url: String,
    pub access_token: String,
    pub user_id: String,
    pub user_name: String,
    pub token_request_id: String,
    pub token_request_secret: String,
    pub token_request_kind: String,
    pub auto_backup_enabled: bool,
    pub auto_backup_interval_minutes: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            access_token: String::new(),
            user_id: String::new(),
            user_name: String::new(),
            token_request_id: String::new(),
            token_request_secret: String::new(),
            token_request_kind: String::new(),
            auto_backup_enabled: false,
            auto_backup_interval_minutes: DEFAULT_AUTO_BACKUP_INTERVAL_MINUTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataSnapshot {
    pub schema_version: u32,
    pub exported_at: i64,
    pub app_version: String,
    #[serde(default)]
    pub device_profile: DeviceProfile,
    pub settings: AppSettings,
    pub workshop: WorkshopState,
    pub layout: LayoutState,
    pub work_logs: WorkLogBook,
    pub focus_sessions: FocusSessionBook,
    pub achievements: AchievementBook,
    pub notes: NoteBook,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DeviceProfile {
    pub captured_at: i64,
    pub cpu_name: Option<String>,
    pub gpu_name: Option<String>,
    pub cpu_physical_core_count: Option<u32>,
    pub cpu_logical_core_count: Option<u32>,
    pub total_memory_bytes: Option<u64>,
    pub gpu_memory_total_bytes: Option<u64>,
    pub inventory: HardwareDeviceInventory,
}

impl From<&HardwareSnapshot> for DeviceProfile {
    fn from(snapshot: &HardwareSnapshot) -> Self {
        Self {
            captured_at: snapshot.timestamp,
            cpu_name: snapshot.cpu_name.clone(),
            gpu_name: snapshot.gpu_name.clone(),
            cpu_physical_core_count: snapshot.cpu_physical_core_count,
            cpu_logical_core_count: snapshot.cpu_logical_core_count,
            total_memory_bytes: snapshot.total_memory_bytes,
            gpu_memory_total_bytes: snapshot.gpu_memory_total_bytes,
            inventory: snapshot.device_inventory.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudEnvelope {
    revision: String,
    stored_at: String,
    #[serde(default)]
    restore_request_id: Option<String>,
    data: UserDataSnapshot,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    revision: String,
    stored_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncResult {
    pub revision: String,
    pub stored_at: String,
    pub item_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenRequestEnvelope {
    request_id: String,
    #[serde(default = "registration_request_kind")]
    kind: String,
    #[serde(default)]
    user_id: Option<String>,
    user_name: String,
    requested_at: String,
    decided_at: Option<String>,
    status: String,
    claim_secret: Option<String>,
    access_token: Option<String>,
}

fn registration_request_kind() -> String {
    "registration".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequestResult {
    pub kind: String,
    pub status: String,
    pub user_id: Option<String>,
    pub user_name: String,
    pub requested_at: String,
    pub decided_at: Option<String>,
    pub initial_sync: Option<CloudSyncResult>,
    pub initial_sync_error: Option<String>,
}

#[tauri::command]
pub fn get_sync_config(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    state.storage.load_sync_config()
}

#[tauri::command]
pub fn update_sync_config(
    config: SyncConfig,
    state: State<'_, AppState>,
) -> Result<SyncConfig, String> {
    let normalized = normalize_config(config)?;
    state.storage.save_sync_config(&normalized)?;
    Ok(normalized)
}

#[tauri::command]
pub async fn request_access_token(
    config: SyncConfig,
    state: State<'_, AppState>,
) -> Result<TokenRequestResult, String> {
    let mut config = normalize_config(config)?;
    if config.server_url.is_empty() {
        return Err("请先填写同步服务器地址".to_string());
    }
    if config.user_name.is_empty() {
        return Err("请填写用于管理员识别的用户名".to_string());
    }
    if !config.access_token.is_empty() {
        return Err("当前客户端已经保存访问令牌，无需重复申请".to_string());
    }
    if !config.token_request_id.is_empty() {
        return Err("当前已有等待管理员处理的令牌申请".to_string());
    }

    let endpoint = api_endpoint(&config.server_url, "/v1/token-requests")?;
    let device_id = state.settings.read().await.cat_id.clone();
    let response = http_client()?
        .post(endpoint)
        .json(&serde_json::json!({
            "userName": config.user_name,
            "deviceId": device_id,
        }))
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let request = response
        .json::<TokenRequestEnvelope>()
        .await
        .map_err(|error| format!("同步服务返回了无效申请响应: {error}"))?;
    let claim_secret = request
        .claim_secret
        .clone()
        .filter(|secret| secret.len() >= 32)
        .ok_or_else(|| "同步服务未返回有效的申请凭据".to_string())?;

    config.user_name = request.user_name.clone();
    config.token_request_id = request.request_id.clone();
    config.token_request_secret = claim_secret;
    config.token_request_kind = request.kind.clone();
    state.storage.save_sync_config(&config)?;

    Ok(token_request_result(request, None, None))
}

#[tauri::command]
pub async fn request_access_token_recovery(
    config: SyncConfig,
    state: State<'_, AppState>,
) -> Result<TokenRequestResult, String> {
    let mut config = normalize_config(config)?;
    if config.server_url.is_empty() {
        return Err("请先填写同步服务器地址".to_string());
    }
    if config.user_id.is_empty() {
        return Err("请填写需要恢复的用户唯一 ID".to_string());
    }
    if !config.access_token.is_empty() {
        return Err("当前客户端已经保存访问令牌，无需恢复".to_string());
    }
    if !config.token_request_id.is_empty() {
        return Err("当前已有等待管理员处理的令牌申请".to_string());
    }

    let endpoint = api_endpoint(&config.server_url, "/v1/token-recoveries")?;
    let device_id = state.settings.read().await.cat_id.clone();
    let response = http_client()?
        .post(endpoint)
        .json(&serde_json::json!({
            "userId": config.user_id,
            "deviceId": device_id,
        }))
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let request = response
        .json::<TokenRequestEnvelope>()
        .await
        .map_err(|error| format!("同步服务返回了无效恢复申请: {error}"))?;
    let claim_secret = request
        .claim_secret
        .clone()
        .filter(|secret| secret.len() >= 32)
        .ok_or_else(|| "同步服务未返回有效的申请凭据".to_string())?;

    config.user_name = request.user_name.clone();
    config.user_id = request.user_id.clone().unwrap_or(config.user_id);
    config.token_request_id = request.request_id.clone();
    config.token_request_secret = claim_secret;
    config.token_request_kind = request.kind.clone();
    state.storage.save_sync_config(&config)?;

    Ok(token_request_result(request, None, None))
}

#[tauri::command]
pub async fn check_access_token_request(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TokenRequestResult, String> {
    let _guard = TOKEN_REQUEST_CHECK_LOCK.lock().await;
    check_token_request(&state, &app).await
}

async fn check_token_request(
    state: &AppState,
    app: &AppHandle,
) -> Result<TokenRequestResult, String> {
    let mut config = normalize_config(state.storage.load_sync_config()?)?;
    if config.token_request_id.is_empty() || config.token_request_secret.is_empty() {
        return Err("当前没有等待处理的令牌申请".to_string());
    }
    let endpoint = api_endpoint(
        &config.server_url,
        &format!("/v1/token-requests/{}", config.token_request_id),
    )?;
    let response = http_client()?
        .get(endpoint)
        .bearer_auth(&config.token_request_secret)
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let request = response
        .json::<TokenRequestEnvelope>()
        .await
        .map_err(|error| format!("同步服务返回了无效申请状态: {error}"))?;

    if request.status == "approved" {
        let access_token = request
            .access_token
            .as_ref()
            .filter(|token| token.len() >= 32)
            .ok_or_else(|| "管理员已批准申请，但服务器未返回有效令牌".to_string())?;
        config.access_token = access_token.clone();
        if let Some(user_id) = request.user_id.as_ref() {
            config.user_id = user_id.clone();
        }
        config.token_request_id.clear();
        config.token_request_secret.clear();
        config.token_request_kind.clear();
        state.storage.save_sync_config(&config)?;
        let sync = if request.kind == "recovery" {
            download_with_config(state, app, &config).await
        } else {
            upload_with_config(state, &config).await
        };
        let (initial_sync, initial_sync_error) = match sync {
            Ok(result) => (Some(result), None),
            Err(error) => (None, Some(error)),
        };
        return Ok(token_request_result(
            request,
            initial_sync,
            initial_sync_error,
        ));
    }

    if request.status == "rejected" {
        config.token_request_id.clear();
        config.token_request_secret.clear();
        config.token_request_kind.clear();
        state.storage.save_sync_config(&config)?;
    }
    Ok(token_request_result(request, None, None))
}

pub fn start_token_request_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let state = app.state::<AppState>();
            let _guard = TOKEN_REQUEST_CHECK_LOCK.lock().await;
            let config = match state.storage.load_sync_config().and_then(normalize_config) {
                Ok(config) => config,
                Err(error) => {
                    tracing::warn!("failed to read cloud sync polling state: {error}");
                    continue;
                }
            };
            if !config.server_url.is_empty() && !config.access_token.is_empty() {
                match apply_pending_restore(&state, &app, &config).await {
                    Ok(Some(result)) => tracing::info!(
                        revision = %result.revision,
                        "server-pushed cloud restore applied"
                    ),
                    Ok(None) => {}
                    Err(error) => tracing::warn!("server-pushed cloud restore failed: {error}"),
                }
                continue;
            }
            if config.token_request_id.is_empty() {
                continue;
            }
            if let Err(error) = check_token_request(&state, &app).await {
                tracing::warn!("automatic token request check failed: {error}");
            }
        }
    });
}

pub fn start_auto_backup_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut schedule: Option<AutoBackupSchedule> = None;
        let mut poll = tokio::time::interval(Duration::from_secs(30));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            poll.tick().await;
            let state = app.state::<AppState>();
            let config = match state.storage.load_sync_config().and_then(normalize_config) {
                Ok(config) => config,
                Err(error) => {
                    schedule = None;
                    tracing::warn!("failed to read automatic backup config: {error}");
                    continue;
                }
            };
            let Some(interval) = auto_backup_interval(&config) else {
                schedule = None;
                continue;
            };

            if !automatic_backup_due(&mut schedule, &config, interval, Instant::now()) {
                continue;
            }

            match upload_with_config(&state, &config).await {
                Ok(result) => tracing::info!(
                    revision = %result.revision,
                    "automatic cloud backup uploaded"
                ),
                Err(error) => tracing::warn!("automatic cloud backup failed: {error}"),
            }
        }
    });
}

fn automatic_backup_due(
    schedule: &mut Option<AutoBackupSchedule>,
    config: &SyncConfig,
    interval: Duration,
    now: Instant,
) -> bool {
    let schedule_changed = schedule.as_ref().is_none_or(
        |(server_url, access_token, user_name, interval_minutes, _)| {
            server_url != &config.server_url
                || access_token != &config.access_token
                || user_name != &config.user_name
                || *interval_minutes != config.auto_backup_interval_minutes
        },
    );
    if schedule_changed {
        *schedule = Some((
            config.server_url.clone(),
            config.access_token.clone(),
            config.user_name.clone(),
            config.auto_backup_interval_minutes,
            now + interval,
        ));
        return true;
    }

    let Some((_, _, _, _, next_upload_at)) = schedule.as_mut() else {
        return false;
    };
    if now < *next_upload_at {
        return false;
    }
    *next_upload_at = now + interval;
    true
}

#[tauri::command]
pub async fn upload_user_data(state: State<'_, AppState>) -> Result<CloudSyncResult, String> {
    let config = configured(&state)?;
    upload_with_config(&state, &config).await
}

async fn upload_with_config(
    state: &AppState,
    config: &SyncConfig,
) -> Result<CloudSyncResult, String> {
    let _guard = SYNC_LOCK.lock().await;
    let endpoint = snapshot_endpoint(config)?;
    let snapshot = snapshot_from_state(state).await;
    validate_snapshot(&snapshot)?;

    let response = http_client()?
        .put(endpoint)
        .bearer_auth(&config.access_token)
        .json(&snapshot)
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let uploaded = response
        .json::<UploadResponse>()
        .await
        .map_err(|error| format!("同步服务返回了无效响应: {error}"))?;

    Ok(CloudSyncResult {
        revision: uploaded.revision,
        stored_at: uploaded.stored_at,
        item_count: snapshot_item_count(&snapshot),
    })
}

fn token_request_result(
    request: TokenRequestEnvelope,
    initial_sync: Option<CloudSyncResult>,
    initial_sync_error: Option<String>,
) -> TokenRequestResult {
    TokenRequestResult {
        kind: request.kind,
        status: request.status,
        user_id: request.user_id,
        user_name: request.user_name,
        requested_at: request.requested_at,
        decided_at: request.decided_at,
        initial_sync,
        initial_sync_error,
    }
}

#[tauri::command]
pub async fn download_user_data(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<CloudSyncResult, String> {
    let config = configured(&state)?;
    download_with_config(&state, &app, &config).await
}

async fn download_with_config(
    state: &AppState,
    app: &AppHandle,
    config: &SyncConfig,
) -> Result<CloudSyncResult, String> {
    let _guard = SYNC_LOCK.lock().await;
    let endpoint = snapshot_endpoint(config)?;
    let response = http_client()?
        .get(endpoint)
        .bearer_auth(&config.access_token)
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let envelope = response
        .json::<CloudEnvelope>()
        .await
        .map_err(|error| format!("云端备份格式无效: {error}"))?;
    restore_cloud_envelope(state, app, envelope).await
}

async fn apply_pending_restore(
    state: &AppState,
    app: &AppHandle,
    config: &SyncConfig,
) -> Result<Option<CloudSyncResult>, String> {
    let _guard = SYNC_LOCK.lock().await;
    let endpoint = api_endpoint(&config.server_url, "/v1/snapshot/restore-pending")?;
    let response = http_client()?
        .get(endpoint)
        .bearer_auth(&config.access_token)
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    if matches!(
        response.status(),
        StatusCode::NO_CONTENT | StatusCode::NOT_FOUND
    ) {
        return Ok(None);
    }
    let response = require_success(response).await?;
    let envelope = response
        .json::<CloudEnvelope>()
        .await
        .map_err(|error| format!("服务端推送的备份格式无效: {error}"))?;
    let request_id = envelope
        .restore_request_id
        .clone()
        .ok_or_else(|| "服务端推送缺少还原任务 ID".to_string())?;
    let result = restore_cloud_envelope(state, app, envelope).await?;

    let ack_endpoint = api_endpoint(
        &config.server_url,
        &format!("/v1/snapshot/restore-pending/{request_id}/ack"),
    )?;
    let response = http_client()?
        .post(ack_endpoint)
        .bearer_auth(&config.access_token)
        .send()
        .await
        .map_err(|error| format!("备份已还原，但无法通知同步服务: {error}"))?;
    require_success(response).await?;
    Ok(Some(result))
}

async fn restore_cloud_envelope(
    state: &AppState,
    app: &AppHandle,
    mut envelope: CloudEnvelope,
) -> Result<CloudSyncResult, String> {
    validate_snapshot(&envelope.data)?;
    envelope.data.settings.enforce_minimal_mode_constraints();
    let device_profile = current_device_profile(state).await;

    let mut settings = state.settings.write().await;
    let mut workshop = state.workshop.write().await;
    let mut layout = state.layout.write().await;
    let mut work_logs = state.work_logs.write().await;
    let mut focus_sessions = state.focus_sessions.write().await;
    let mut achievements = state.achievements.write().await;
    let mut notes = state.notes.write().await;

    let mut local_work_logs = state.storage.load_all_work_logs()?;
    merge_work_log_entries(&mut local_work_logs.entries, work_logs.entries.clone());
    let previous = UserDataSnapshot {
        schema_version: 1,
        exported_at: current_timestamp_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_profile,
        settings: settings.clone(),
        workshop: workshop.clone(),
        layout: layout.clone(),
        work_logs: local_work_logs,
        focus_sessions: focus_sessions.clone(),
        achievements: achievements.clone(),
        notes: notes.clone(),
    };
    let merged = merge_user_data_snapshots(&previous, &envelope.data);
    validate_snapshot(&merged)?;
    state
        .storage
        .restore_user_data_snapshot(&merged, &previous)?;
    let hot_work_logs = state.storage.load_or_create_work_logs()?;

    *settings = merged.settings.clone();
    *workshop = merged.workshop.clone();
    *layout = merged.layout.clone();
    *work_logs = hot_work_logs;
    *focus_sessions = merged.focus_sessions.clone();
    *achievements = merged.achievements.clone();
    *notes = merged.notes.clone();

    let launch_at_startup = settings.launch_at_startup;
    drop(settings);
    drop(workshop);
    drop(layout);
    drop(work_logs);
    drop(focus_sessions);
    drop(achievements);
    drop(notes);

    if let Err(error) = crate::commands::sync_launch_at_startup(launch_at_startup) {
        tracing::warn!(
            "cloud data restored but startup registration could not be updated: {error}"
        );
    }
    emit_restored_state(app, &merged);

    Ok(CloudSyncResult {
        revision: envelope.revision,
        stored_at: envelope.stored_at,
        item_count: snapshot_item_count(&merged),
    })
}

async fn snapshot_from_state(state: &AppState) -> UserDataSnapshot {
    let device_profile = current_device_profile(state).await;
    let settings = state.settings.read().await;
    let workshop = state.workshop.read().await;
    let layout = state.layout.read().await;
    let work_logs = state.work_logs.read().await;
    let focus_sessions = state.focus_sessions.read().await;
    let achievements = state.achievements.read().await;
    let notes = state.notes.read().await;

    let mut all_work_logs = state.storage.load_all_work_logs().unwrap_or_else(|error| {
        tracing::warn!("failed to include archived work logs in cloud backup: {error}");
        work_logs.clone()
    });
    merge_work_log_entries(&mut all_work_logs.entries, work_logs.entries.clone());

    UserDataSnapshot {
        schema_version: 1,
        exported_at: current_timestamp_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_profile,
        settings: settings.clone(),
        workshop: workshop.clone(),
        layout: layout.clone(),
        work_logs: all_work_logs,
        focus_sessions: focus_sessions.clone(),
        achievements: achievements.clone(),
        notes: notes.clone(),
    }
}

fn merge_user_data_snapshots(
    local: &UserDataSnapshot,
    remote: &UserDataSnapshot,
) -> UserDataSnapshot {
    let mut settings = remote.settings.clone();
    settings.cat_id = local.settings.cat_id.clone();
    settings.launch_at_startup = local.settings.launch_at_startup;
    settings.cat_window_x = local.settings.cat_window_x;
    settings.cat_window_y = local.settings.cat_window_y;
    settings.monitor_bar_x = local.settings.monitor_bar_x;
    settings.monitor_bar_y = local.settings.monitor_bar_y;
    settings.integrated_hardware_monitor_enabled =
        local.settings.integrated_hardware_monitor_enabled;
    settings.memory_last_release = local.settings.memory_last_release.clone();
    settings.enforce_minimal_mode_constraints();

    let workshop = if remote.workshop.last_production_time > local.workshop.last_production_time {
        remote.workshop.clone()
    } else {
        local.workshop.clone()
    };

    UserDataSnapshot {
        schema_version: 1,
        exported_at: local.exported_at.max(remote.exported_at),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_profile: local.device_profile.clone(),
        settings,
        workshop,
        layout: local.layout.clone(),
        work_logs: merge_work_log_books(&local.work_logs, &remote.work_logs),
        focus_sessions: merge_focus_sessions(&local.focus_sessions, &remote.focus_sessions),
        achievements: merge_achievements(&local.achievements, &remote.achievements),
        notes: merge_notes(&local.notes, &remote.notes),
    }
}

fn merge_notes(local: &NoteBook, remote: &NoteBook) -> NoteBook {
    let mut by_id = BTreeMap::<String, Note>::new();
    for note in local.notes.iter().chain(&remote.notes) {
        let replace = by_id
            .get(&note.id)
            .is_none_or(|current| note.updated_at > current.updated_at);
        if replace {
            by_id.insert(note.id.clone(), note.clone());
        }
    }
    NoteBook {
        schema_version: local.schema_version.max(remote.schema_version),
        notes: by_id.into_values().collect(),
    }
}

fn merge_focus_sessions(local: &FocusSessionBook, remote: &FocusSessionBook) -> FocusSessionBook {
    let mut by_id = BTreeMap::<String, FocusSession>::new();
    for session in local.sessions.iter().chain(&remote.sessions) {
        let replace = by_id
            .get(&session.id)
            .is_none_or(|current| focus_session_rank(session) > focus_session_rank(current));
        if replace {
            by_id.insert(session.id.clone(), session.clone());
        }
    }
    let mut sessions = by_id.into_values().collect::<Vec<_>>();
    sessions.sort_by_key(|session| session.started_at);
    FocusSessionBook {
        schema_version: local.schema_version.max(remote.schema_version),
        sessions,
    }
}

fn focus_session_rank(session: &FocusSession) -> (u8, i64, u32) {
    let status = match session.status {
        FocusSessionStatus::Active => 0,
        FocusSessionStatus::Abandoned => 1,
        FocusSessionStatus::Completed => 2,
    };
    (
        status,
        session.ended_at.unwrap_or(session.started_at),
        session.distraction_count,
    )
}

fn merge_work_log_books(local: &WorkLogBook, remote: &WorkLogBook) -> WorkLogBook {
    let mut entries = local.entries.clone();
    merge_work_log_entries(&mut entries, remote.entries.clone());
    WorkLogBook {
        schema_version: WORK_LOG_BOOK_SCHEMA_VERSION,
        entries,
    }
}

fn merge_work_log_entries(
    target: &mut BTreeMap<String, WorkLogEntry>,
    incoming: BTreeMap<String, WorkLogEntry>,
) {
    for (date, entry) in incoming {
        if target
            .get(&date)
            .is_none_or(|current| entry.updated_at > current.updated_at)
        {
            target.insert(date, entry);
        }
    }
}

fn merge_achievements(local: &AchievementBook, remote: &AchievementBook) -> AchievementBook {
    let mut events = BTreeMap::new();
    for event in local.events.iter().chain(&remote.events) {
        events
            .entry(event.idempotency_key.clone())
            .or_insert_with(|| event.clone());
    }
    let mut counters = local.counters.clone();
    for (key, value) in &remote.counters {
        counters
            .entry(key.clone())
            .and_modify(|current| *current = current.max(*value))
            .or_insert(*value);
    }
    let mut daily_rollups = local.daily_rollups.clone();
    for (date, incoming) in &remote.daily_rollups {
        daily_rollups
            .entry(date.clone())
            .and_modify(|current| merge_daily_rollup(current, incoming))
            .or_insert_with(|| incoming.clone());
    }
    let mut distinct_values = local.distinct_values.clone();
    for (key, values) in &remote.distinct_values {
        distinct_values
            .entry(key.clone())
            .or_default()
            .extend(values.iter().cloned());
    }
    let mut unlocks = local.unlocks.clone();
    for (key, unlock) in &remote.unlocks {
        unlocks.entry(key.clone()).or_insert_with(|| unlock.clone());
    }
    let mut notifications = local.notifications.clone();
    for (key, notification) in &remote.notifications {
        notifications
            .entry(key.clone())
            .and_modify(|current| {
                if (notification.seen_at.is_some() && current.seen_at.is_none())
                    || notification.created_at > current.created_at
                {
                    *current = notification.clone();
                }
            })
            .or_insert_with(|| notification.clone());
    }
    let notification_queue = local
        .notification_queue
        .iter()
        .chain(&remote.notification_queue)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let weekly_goal_plan = match (&local.weekly_goal_plan, &remote.weekly_goal_plan) {
        (Some(left), Some(right)) if right.generated_at > left.generated_at => Some(right.clone()),
        (Some(left), _) => Some(left.clone()),
        (_, Some(right)) => Some(right.clone()),
        _ => None,
    };

    let mut events = events.into_values().collect::<Vec<_>>();
    events.sort_by_key(|event| (event.occurred_at, event.received_at));
    const MAX_MERGED_ACHIEVEMENT_EVENTS: usize = 2_000;
    if events.len() > MAX_MERGED_ACHIEVEMENT_EVENTS {
        events.drain(0..events.len() - MAX_MERGED_ACHIEVEMENT_EVENTS);
    }
    let idempotency_keys = events
        .iter()
        .map(|event| event.idempotency_key.clone())
        .collect();

    AchievementBook {
        schema_version: ACHIEVEMENT_BOOK_SCHEMA_VERSION,
        events,
        counters,
        daily_rollups,
        distinct_values,
        unlocks,
        notifications,
        notification_queue,
        idempotency_keys,
        weekly_goal_plan,
    }
}

fn merge_daily_rollup(current: &mut AchievementDailyRollup, incoming: &AchievementDailyRollup) {
    current.active_seconds = current.active_seconds.max(incoming.active_seconds);
    current.high_load_seconds = current.high_load_seconds.max(incoming.high_load_seconds);
    current.thermal_warning_seconds = current
        .thermal_warning_seconds
        .max(incoming.thermal_warning_seconds);
    current.active_00_05_seconds = current
        .active_00_05_seconds
        .max(incoming.active_00_05_seconds);
    current.low_power_mode_enabled_seconds = current
        .low_power_mode_enabled_seconds
        .max(incoming.low_power_mode_enabled_seconds);
    current.report_generated |= incoming.report_generated;
    current.report_score = max_option(current.report_score, incoming.report_score);
    current.report_day_type = current
        .report_day_type
        .clone()
        .or_else(|| incoming.report_day_type.clone());
    current.rarity_tier = current
        .rarity_tier
        .clone()
        .or_else(|| incoming.rarity_tier.clone());
    current.rarity_rank = max_option(current.rarity_rank, incoming.rarity_rank);
    current.rarity_score = max_option(current.rarity_score, incoming.rarity_score);
    current.title_family = current
        .title_family
        .clone()
        .or_else(|| incoming.title_family.clone());
    current.title_level = max_option(current.title_level, incoming.title_level);
    current.title_progress = max_option(current.title_progress, incoming.title_progress);
    current.storage_corruption_rebuilt_count = current
        .storage_corruption_rebuilt_count
        .max(incoming.storage_corruption_rebuilt_count);
}

fn max_option(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

async fn current_device_profile(state: &AppState) -> DeviceProfile {
    state
        .last_snapshot
        .read()
        .await
        .as_ref()
        .map(DeviceProfile::from)
        .unwrap_or_default()
}

fn configured(state: &AppState) -> Result<SyncConfig, String> {
    let config = state.storage.load_sync_config()?;
    if config.server_url.is_empty() || config.access_token.is_empty() {
        return Err("请先保存同步服务器地址和访问令牌".to_string());
    }
    normalize_config(config)
}

fn normalize_config(mut config: SyncConfig) -> Result<SyncConfig, String> {
    config.server_url = config.server_url.trim().trim_end_matches('/').to_string();
    config.access_token = config.access_token.trim().to_string();
    config.user_id = config.user_id.trim().to_string();
    config.user_name = config.user_name.trim().to_string();
    config.token_request_id = config.token_request_id.trim().to_string();
    config.token_request_secret = config.token_request_secret.trim().to_string();
    config.token_request_kind = config.token_request_kind.trim().to_string();
    if !config.token_request_id.is_empty() && config.token_request_kind.is_empty() {
        config.token_request_kind = "registration".to_string();
    }
    if !(MIN_AUTO_BACKUP_INTERVAL_MINUTES..=MAX_AUTO_BACKUP_INTERVAL_MINUTES)
        .contains(&config.auto_backup_interval_minutes)
    {
        return Err(format!(
            "自动备份间隔需要设置为 {MIN_AUTO_BACKUP_INTERVAL_MINUTES} 到 {MAX_AUTO_BACKUP_INTERVAL_MINUTES} 分钟"
        ));
    }
    if config.server_url.is_empty()
        && config.access_token.is_empty()
        && config.user_id.is_empty()
        && config.user_name.is_empty()
        && config.token_request_id.is_empty()
        && config.token_request_secret.is_empty()
        && config.token_request_kind.is_empty()
    {
        if config.auto_backup_enabled {
            return Err("启用自动备份前，请先保存服务器地址和访问令牌".to_string());
        }
        return Ok(config);
    }
    if config.server_url.is_empty() {
        return Err("请填写同步服务器地址".to_string());
    }
    snapshot_endpoint(&config)?;
    if !config.access_token.is_empty() && config.access_token.len() < 32 {
        return Err("访问令牌至少需要 32 个字符".to_string());
    }
    if !config.user_id.is_empty()
        && (config.user_id.len() < 32
            || config.user_id.len() > 64
            || !config
                .user_id
                .chars()
                .all(|character| character.is_ascii_hexdigit() || character == '-'))
    {
        return Err("用户唯一 ID 无效".to_string());
    }
    if config.user_name.chars().count() > 40
        || config
            .user_name
            .chars()
            .any(|character| character.is_control())
    {
        return Err("用户名需要控制在 40 个可见字符以内".to_string());
    }
    if config.token_request_id.is_empty() != config.token_request_secret.is_empty() {
        return Err("本机保存的令牌申请状态不完整，请重新申请".to_string());
    }
    if !config.token_request_secret.is_empty() && config.token_request_secret.len() < 32 {
        return Err("本机保存的令牌申请凭据无效，请重新申请".to_string());
    }
    if !config.token_request_kind.is_empty()
        && !matches!(
            config.token_request_kind.as_str(),
            "registration" | "recovery"
        )
    {
        return Err("本机保存的令牌申请类型无效，请重新申请".to_string());
    }
    if config.token_request_id.is_empty() != config.token_request_kind.is_empty() {
        return Err("本机保存的令牌申请类型不完整，请重新申请".to_string());
    }
    if !config.access_token.is_empty() {
        config.token_request_id.clear();
        config.token_request_secret.clear();
        config.token_request_kind.clear();
    }
    if config.auto_backup_enabled
        && (config.server_url.is_empty() || config.access_token.is_empty())
    {
        return Err("启用自动备份前，请先保存服务器地址和访问令牌".to_string());
    }
    Ok(config)
}

fn auto_backup_interval(config: &SyncConfig) -> Option<Duration> {
    (config.auto_backup_enabled && !config.server_url.is_empty() && !config.access_token.is_empty())
        .then(|| Duration::from_secs(config.auto_backup_interval_minutes * 60))
}

fn snapshot_endpoint(config: &SyncConfig) -> Result<Url, String> {
    api_endpoint(&config.server_url, "/v1/snapshot")
}

fn api_endpoint(server_url: &str, endpoint_path: &str) -> Result<Url, String> {
    let mut url = Url::parse(server_url).map_err(|_| "同步服务器地址无效".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("同步服务器地址必须使用 HTTP 或 HTTPS".to_string());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("同步服务器地址不能包含账号、查询参数或片段".to_string());
    }
    let path = format!("{}{}", url.path().trim_end_matches('/'), endpoint_path);
    url.set_path(&path);
    Ok(url)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("无法创建同步请求: {error}"))
}

async fn require_success(response: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let detail = response.text().await.unwrap_or_default();
    let detail = detail.chars().take(300).collect::<String>();
    let message = match status {
        StatusCode::UNAUTHORIZED => "访问令牌无效".to_string(),
        StatusCode::NOT_FOUND => "云端还没有可恢复的备份".to_string(),
        _ if detail.is_empty() => format!("同步服务返回错误 {status}"),
        _ => format!("同步服务返回错误 {status}: {detail}"),
    };
    Err(message)
}

fn validate_snapshot(snapshot: &UserDataSnapshot) -> Result<(), String> {
    if snapshot.schema_version != 1 {
        return Err("不支持的云端备份版本".to_string());
    }
    let workshop = &snapshot.workshop;
    if workshop.workshop_level < 1
        || workshop.parts < 0.0
        || workshop.insight < 0.0
        || !workshop.parts.is_finite()
        || !workshop.insight.is_finite()
        || !workshop.today_parts.is_finite()
        || !workshop.today_insight.is_finite()
    {
        return Err("云端工坊数据无效，已停止恢复".to_string());
    }
    if !snapshot.layout.cat_window_x.is_finite()
        || !snapshot.layout.cat_window_y.is_finite()
        || !snapshot.layout.monitor_bar_x.is_finite()
        || !snapshot.layout.monitor_bar_y.is_finite()
    {
        return Err("云端窗口配置无效，已停止恢复".to_string());
    }
    let mut note_ids = HashSet::new();
    if snapshot
        .notes
        .notes
        .iter()
        .any(|note| note.id.is_empty() || !note_ids.insert(&note.id))
    {
        return Err("云端笔记包含空 ID 或重复 ID，已停止恢复".to_string());
    }
    Ok(())
}

fn snapshot_item_count(snapshot: &UserDataSnapshot) -> usize {
    snapshot.notes.notes.len()
        + snapshot.work_logs.entries.len()
        + snapshot.focus_sessions.sessions.len()
        + snapshot.achievements.unlocks.len()
}

fn emit_restored_state(app: &AppHandle, snapshot: &UserDataSnapshot) {
    for result in [
        app.emit(SETTINGS_UPDATED, snapshot.settings.clone()),
        app.emit(WORKSHOP_UPDATED, snapshot.workshop.clone()),
        app.emit(FOCUS_SESSION_UPDATED, snapshot.focus_sessions.clone()),
        app.emit(NOTES_UPDATED, snapshot.notes.clone()),
    ] {
        if let Err(error) = result {
            tracing::warn!("failed to emit restored cloud state: {error}");
        }
    }
    if let Err(error) = app.emit(ACHIEVEMENT_PROGRESS_UPDATED, ()) {
        tracing::warn!("failed to emit restored achievement state: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        auto_backup_interval, automatic_backup_due, merge_user_data_snapshots, normalize_config,
        DeviceProfile, SyncConfig, UserDataSnapshot,
    };
    use crate::{
        achievements::{
            ensure_weekly_goals, load_seed_definitions, AchievementBook, AchievementEventRecord,
            WeeklyGoalPlan, ACHIEVEMENT_BOOK_SCHEMA_VERSION,
        },
        models::{
            AppSettings, FocusSession, FocusSessionBook, FocusSessionStatus, HardwareSnapshot,
            LayoutState, Note, NoteBook, ProcessUsageSnapshot, WorkLogBook, WorkLogEntry,
            WorkshopState,
        },
    };

    #[test]
    fn remote_http_and_https_are_allowed_but_other_schemes_are_rejected() {
        let token = "x".repeat(32);
        assert!(normalize_config(SyncConfig {
            server_url: "http://example.com".to_string(),
            access_token: token.clone(),
            ..Default::default()
        })
        .is_ok());
        assert!(normalize_config(SyncConfig {
            server_url: "https://sync.example.com".to_string(),
            access_token: token.clone(),
            ..Default::default()
        })
        .is_ok());
        assert!(normalize_config(SyncConfig {
            server_url: "ftp://example.com".to_string(),
            access_token: token,
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn empty_config_can_disable_sync() {
        assert!(normalize_config(SyncConfig::default()).is_ok());
        assert!(normalize_config(SyncConfig {
            server_url: "https://sync.example.com".to_string(),
            access_token: String::new(),
            ..Default::default()
        })
        .is_ok());
    }

    #[test]
    fn pending_request_credentials_must_be_complete() {
        assert!(normalize_config(SyncConfig {
            server_url: "https://sync.example.com".to_string(),
            token_request_id: "request-id".to_string(),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn automatic_backup_defaults_to_disabled_with_30_minute_interval() {
        let config = serde_json::from_str::<SyncConfig>("{}").unwrap();

        assert!(!config.auto_backup_enabled);
        assert_eq!(config.auto_backup_interval_minutes, 30);
        assert!(auto_backup_interval(&config).is_none());
    }

    #[test]
    fn automatic_backup_requires_credentials_and_uses_configured_interval() {
        let token = "x".repeat(32);
        assert!(normalize_config(SyncConfig {
            auto_backup_enabled: true,
            ..Default::default()
        })
        .is_err());

        let config = normalize_config(SyncConfig {
            server_url: "https://sync.example.com".to_string(),
            access_token: token,
            auto_backup_enabled: true,
            auto_backup_interval_minutes: 30,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            auto_backup_interval(&config),
            Some(Duration::from_secs(30 * 60))
        );
    }

    #[test]
    fn automatic_backup_uploads_immediately_then_waits_for_the_interval() {
        let config = SyncConfig {
            server_url: "https://sync.example.com".to_string(),
            access_token: "x".repeat(32),
            auto_backup_enabled: true,
            auto_backup_interval_minutes: 30,
            ..Default::default()
        };
        let interval = auto_backup_interval(&config).unwrap();
        let started_at = Instant::now();
        let mut schedule = None;

        assert!(automatic_backup_due(
            &mut schedule,
            &config,
            interval,
            started_at
        ));
        assert!(!automatic_backup_due(
            &mut schedule,
            &config,
            interval,
            started_at + Duration::from_secs(60)
        ));
        assert!(automatic_backup_due(
            &mut schedule,
            &config,
            interval,
            started_at + interval
        ));
    }

    #[test]
    fn device_profile_excludes_live_metrics_and_processes() {
        let snapshot = HardwareSnapshot {
            cpu_name: Some("Test CPU".to_string()),
            gpu_name: Some("Test GPU".to_string()),
            cpu_usage_percent: Some(82.0),
            processes: vec![ProcessUsageSnapshot {
                pid: 42,
                name: "private-process".to_string(),
                ..ProcessUsageSnapshot::default()
            }],
            ..HardwareSnapshot::default()
        };

        let profile = DeviceProfile::from(&snapshot);
        let json = serde_json::to_string(&profile).unwrap();

        assert_eq!(profile.cpu_name.as_deref(), Some("Test CPU"));
        assert_eq!(profile.gpu_name.as_deref(), Some("Test GPU"));
        assert!(!json.contains("cpuUsagePercent"));
        assert!(!json.contains("private-process"));
        assert!(!json.contains("processes"));
    }

    #[test]
    fn cloud_restore_merges_domains_without_overwriting_newer_local_data() {
        let mut local = test_snapshot(100);
        let mut remote = test_snapshot(200);
        local.settings.cat_id = "LOCAL-CAT".to_string();
        remote.settings.cat_id = "REMOTE-CAT".to_string();
        remote.settings.cat_name = "云端 CoCat".to_string();
        local.notes.notes.push(Note {
            id: "same".to_string(),
            title: "local newer".to_string(),
            updated_at: 300,
            ..Default::default()
        });
        remote.notes.notes.push(Note {
            id: "same".to_string(),
            title: "remote older".to_string(),
            updated_at: 200,
            ..Default::default()
        });
        local.work_logs.entries.insert(
            "2026-09-14".to_string(),
            WorkLogEntry {
                date: "2026-09-14".to_string(),
                updated_at: 300,
                active_seconds: 30,
                ..Default::default()
            },
        );
        remote.work_logs.entries.insert(
            "2026-09-14".to_string(),
            WorkLogEntry {
                date: "2026-09-14".to_string(),
                updated_at: 200,
                active_seconds: 20,
                ..Default::default()
            },
        );
        local.focus_sessions.sessions.push(FocusSession {
            id: "focus-1".to_string(),
            started_at: 100,
            status: FocusSessionStatus::Active,
            ..Default::default()
        });
        remote.focus_sessions.sessions.push(FocusSession {
            id: "focus-1".to_string(),
            started_at: 100,
            ended_at: Some(250),
            status: FocusSessionStatus::Completed,
            ..Default::default()
        });

        let merged = merge_user_data_snapshots(&local, &remote);

        assert_eq!(merged.settings.cat_id, "LOCAL-CAT");
        assert_eq!(merged.settings.cat_name, "云端 CoCat");
        assert_eq!(merged.notes.notes[0].title, "local newer");
        assert_eq!(merged.work_logs.entries["2026-09-14"].active_seconds, 30);
        assert_eq!(
            merged.focus_sessions.sessions[0].status,
            FocusSessionStatus::Completed
        );
    }

    #[test]
    fn repeated_cloud_merge_is_idempotent_for_achievement_events() {
        let local = test_snapshot(100);
        let mut remote = test_snapshot(200);
        remote.achievements.events.push(AchievementEventRecord {
            event_id: "event-1".to_string(),
            event_name: "app.launch".to_string(),
            occurred_at: 1,
            received_at: 1,
            source: "test".to_string(),
            idempotency_key: "launch-1".to_string(),
            payload: serde_json::json!({}),
            app_version: "test".to_string(),
        });

        let once = merge_user_data_snapshots(&local, &remote);
        let twice = merge_user_data_snapshots(&once, &remote);

        assert_eq!(once.achievements.events.len(), 1);
        assert_eq!(twice.achievements.events.len(), 1);
        assert_eq!(twice.achievements.idempotency_keys.len(), 1);
    }

    #[test]
    fn restored_legacy_weekly_goal_plan_is_regenerated_on_next_read() {
        let local = test_snapshot(100);
        let mut remote = test_snapshot(200);
        remote.achievements.schema_version = 2;
        remote.achievements.weekly_goal_plan = Some(WeeklyGoalPlan {
            week_key: "legacy-week".to_string(),
            achievement_ids: vec!["A002".to_string(), "A003".to_string(), "A005".to_string()],
            generated_at: 200,
            ..Default::default()
        });

        let mut merged = merge_user_data_snapshots(&local, &remote).achievements;
        let definitions = load_seed_definitions().unwrap();
        let (goals, changed) = ensure_weekly_goals(&mut merged, &definitions, 200);

        assert_eq!(merged.schema_version, ACHIEVEMENT_BOOK_SCHEMA_VERSION);
        assert!(changed);
        assert_eq!(goals.goals.len(), 3);
        let plan = merged.weekly_goal_plan.unwrap();
        assert!(plan.achievement_ids.is_empty());
        assert_eq!(plan.goal_keys.len(), 3);
        assert_eq!(plan.baselines.len(), 3);
    }

    fn test_snapshot(exported_at: i64) -> UserDataSnapshot {
        UserDataSnapshot {
            schema_version: 1,
            exported_at,
            app_version: "test".to_string(),
            device_profile: Default::default(),
            settings: AppSettings::default(),
            workshop: WorkshopState::default(),
            layout: LayoutState::default(),
            work_logs: WorkLogBook::default(),
            focus_sessions: FocusSessionBook::default(),
            achievements: AchievementBook::default(),
            notes: NoteBook::default(),
        }
    }
}
