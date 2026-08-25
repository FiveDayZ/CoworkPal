use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::{
    achievements::AchievementBook,
    app_state::AppState,
    events::{FOCUS_SESSION_UPDATED, NOTES_UPDATED, SETTINGS_UPDATED, WORKSHOP_UPDATED},
    models::{
        current_timestamp_ms, AppSettings, FocusSessionBook, HardwareDeviceInventory,
        HardwareSnapshot, LayoutState, NoteBook, WorkLogBook, WorkshopState,
    },
};

static TOKEN_REQUEST_CHECK_LOCK: Mutex<()> = Mutex::const_new(());
static UPLOAD_LOCK: Mutex<()> = Mutex::const_new(());

const DEFAULT_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 30;
const MIN_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 5;
const MAX_AUTO_BACKUP_INTERVAL_MINUTES: u64 = 24 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SyncConfig {
    pub server_url: String,
    pub access_token: String,
    pub user_name: String,
    pub token_request_id: String,
    pub token_request_secret: String,
    pub auto_backup_enabled: bool,
    pub auto_backup_interval_minutes: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            access_token: String::new(),
            user_name: String::new(),
            token_request_id: String::new(),
            token_request_secret: String::new(),
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
    user_name: String,
    requested_at: String,
    decided_at: Option<String>,
    status: String,
    claim_secret: Option<String>,
    access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequestResult {
    pub status: String,
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
    state.storage.save_sync_config(&config)?;

    Ok(token_request_result(request, None, None))
}

#[tauri::command]
pub async fn check_access_token_request(
    state: State<'_, AppState>,
) -> Result<TokenRequestResult, String> {
    let _guard = TOKEN_REQUEST_CHECK_LOCK.lock().await;
    check_token_request(&state).await
}

async fn check_token_request(state: &AppState) -> Result<TokenRequestResult, String> {
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
        config.token_request_id.clear();
        config.token_request_secret.clear();
        state.storage.save_sync_config(&config)?;
        let (initial_sync, initial_sync_error) = match upload_with_config(state, &config).await {
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
            let config = match state.storage.load_sync_config() {
                Ok(config) => config,
                Err(error) => {
                    tracing::warn!("failed to read token request state: {error}");
                    continue;
                }
            };
            if config.token_request_id.is_empty() || !config.access_token.is_empty() {
                continue;
            }
            if let Err(error) = check_token_request(&state).await {
                tracing::warn!("automatic token request check failed: {error}");
            }
        }
    });
}

pub fn start_auto_backup_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut schedule: Option<(String, String, u64, Instant)> = None;
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

            let schedule_changed =
                schedule
                    .as_ref()
                    .is_none_or(|(server_url, user_name, interval_minutes, _)| {
                        server_url != &config.server_url
                            || user_name != &config.user_name
                            || *interval_minutes != config.auto_backup_interval_minutes
                    });
            if schedule_changed {
                schedule = Some((
                    config.server_url.clone(),
                    config.user_name.clone(),
                    config.auto_backup_interval_minutes,
                    Instant::now() + interval,
                ));
                continue;
            }

            let Some((_, _, _, next_upload_at)) = schedule.as_mut() else {
                continue;
            };
            if Instant::now() < *next_upload_at {
                continue;
            }
            *next_upload_at = Instant::now() + interval;

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

#[tauri::command]
pub async fn upload_user_data(state: State<'_, AppState>) -> Result<CloudSyncResult, String> {
    let config = configured(&state)?;
    upload_with_config(&state, &config).await
}

async fn upload_with_config(
    state: &AppState,
    config: &SyncConfig,
) -> Result<CloudSyncResult, String> {
    let _guard = UPLOAD_LOCK.lock().await;
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
        status: request.status,
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
    let endpoint = snapshot_endpoint(&config)?;
    let response = http_client()?
        .get(endpoint)
        .bearer_auth(&config.access_token)
        .send()
        .await
        .map_err(|error| format!("无法连接同步服务: {error}"))?;
    let response = require_success(response).await?;
    let mut envelope = response
        .json::<CloudEnvelope>()
        .await
        .map_err(|error| format!("云端备份格式无效: {error}"))?;
    validate_snapshot(&envelope.data)?;
    envelope.data.settings.enforce_minimal_mode_constraints();
    let device_profile = current_device_profile(&state).await;

    let mut settings = state.settings.write().await;
    let mut workshop = state.workshop.write().await;
    let mut layout = state.layout.write().await;
    let mut work_logs = state.work_logs.write().await;
    let mut focus_sessions = state.focus_sessions.write().await;
    let mut achievements = state.achievements.write().await;
    let mut notes = state.notes.write().await;

    let previous = UserDataSnapshot {
        schema_version: 1,
        exported_at: current_timestamp_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_profile,
        settings: settings.clone(),
        workshop: workshop.clone(),
        layout: layout.clone(),
        work_logs: work_logs.clone(),
        focus_sessions: focus_sessions.clone(),
        achievements: achievements.clone(),
        notes: notes.clone(),
    };
    state
        .storage
        .restore_user_data_snapshot(&envelope.data, &previous)?;

    *settings = envelope.data.settings.clone();
    *workshop = envelope.data.workshop.clone();
    *layout = envelope.data.layout.clone();
    *work_logs = envelope.data.work_logs.clone();
    *focus_sessions = envelope.data.focus_sessions.clone();
    *achievements = envelope.data.achievements.clone();
    *notes = envelope.data.notes.clone();

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
    emit_restored_state(&app, &envelope.data);

    Ok(CloudSyncResult {
        revision: envelope.revision,
        stored_at: envelope.stored_at,
        item_count: snapshot_item_count(&envelope.data),
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

    UserDataSnapshot {
        schema_version: 1,
        exported_at: current_timestamp_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_profile,
        settings: settings.clone(),
        workshop: workshop.clone(),
        layout: layout.clone(),
        work_logs: work_logs.clone(),
        focus_sessions: focus_sessions.clone(),
        achievements: achievements.clone(),
        notes: notes.clone(),
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
    config.user_name = config.user_name.trim().to_string();
    config.token_request_id = config.token_request_id.trim().to_string();
    config.token_request_secret = config.token_request_secret.trim().to_string();
    if !(MIN_AUTO_BACKUP_INTERVAL_MINUTES..=MAX_AUTO_BACKUP_INTERVAL_MINUTES)
        .contains(&config.auto_backup_interval_minutes)
    {
        return Err(format!(
            "自动备份间隔需要设置为 {MIN_AUTO_BACKUP_INTERVAL_MINUTES} 到 {MAX_AUTO_BACKUP_INTERVAL_MINUTES} 分钟"
        ));
    }
    if config.server_url.is_empty()
        && config.access_token.is_empty()
        && config.user_name.is_empty()
        && config.token_request_id.is_empty()
        && config.token_request_secret.is_empty()
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
    if !config.access_token.is_empty() {
        config.token_request_id.clear();
        config.token_request_secret.clear();
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
    if url.scheme() != "https" && !(url.scheme() == "http" && is_local_host(&url)) {
        return Err("远程同步服务器必须使用 HTTPS".to_string());
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

fn is_local_host(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"))
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
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{auto_backup_interval, normalize_config, DeviceProfile, SyncConfig};
    use crate::models::{HardwareSnapshot, ProcessUsageSnapshot};

    #[test]
    fn remote_http_is_rejected_but_local_http_is_allowed() {
        let token = "x".repeat(32);
        assert!(normalize_config(SyncConfig {
            server_url: "http://example.com".to_string(),
            access_token: token.clone(),
            ..Default::default()
        })
        .is_err());
        assert!(normalize_config(SyncConfig {
            server_url: "http://127.0.0.1:8080/".to_string(),
            access_token: token,
            ..Default::default()
        })
        .is_ok());
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
}
