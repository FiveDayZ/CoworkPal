use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    achievements::AchievementBook,
    cloud_sync::{SyncConfig, UserDataSnapshot},
    models::{
        AppSettings, FocusSessionBook, LayoutState, NoteBook, WorkLogBook, WorkshopState,
        APP_SETTINGS_SCHEMA_VERSION,
    },
};

const LAST_GOOD_BACKUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(300);

#[derive(Debug)]
pub struct StorageService {
    root: PathBuf,
    corruption_rebuilds: Arc<Mutex<Vec<String>>>,
    /// Cached SMBIOS UUID (the machine's stable hardware id). `query_smbios_uuid`
    /// spawns powershell.exe, which is slow; since the UUID does not change for
    /// the life of the process we compute it at most once per StorageService.
    smbios_uuid: OnceLock<Option<String>>,
}

impl StorageService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = app_data_root()?;
        Self::new_with_root(root)
    }

    pub fn new_with_root(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(root.join("logs"))?;
        fs::create_dir_all(root.join("backups"))?;
        Ok(Self {
            root,
            corruption_rebuilds: Arc::new(Mutex::new(Vec::new())),
            smbios_uuid: OnceLock::new(),
        })
    }

    pub fn load_or_create_settings(&self) -> Result<AppSettings, String> {
        let mut settings = self.load_or_create::<AppSettings>("settings.json")?;
        let smbios_uuid = settings
            .cat_id
            .is_empty()
            .then(|| self.cached_smbios_uuid())
            .flatten();
        let changed = migrate_settings(&mut settings, smbios_uuid);
        let minimal_mode_corrected = settings.enforce_minimal_mode_constraints();
        if changed || minimal_mode_corrected {
            self.save_settings(&settings)?;
        }
        Ok(settings)
    }

    /// Returns the cached SMBIOS UUID, querying powershell only on the first call.
    fn cached_smbios_uuid(&self) -> Option<String> {
        self.smbios_uuid.get_or_init(query_smbios_uuid).clone()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        self.write_json("settings.json", settings)
    }

    pub fn load_or_create_workshop(&self) -> Result<WorkshopState, String> {
        self.load_or_create("save.json")
    }

    pub fn save_workshop(&self, workshop: &WorkshopState) -> Result<(), String> {
        self.write_json("save.json", workshop)
    }

    pub fn load_sync_config(&self) -> Result<SyncConfig, String> {
        self.load_or_create("sync.json")
    }

    pub fn save_sync_config(&self, config: &SyncConfig) -> Result<(), String> {
        self.write_json("sync.json", config)
    }

    pub fn apply_pending_user_data_restore(&self) -> Result<(), String> {
        let path = self.root.join("restore.pending.json");
        if !path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read pending cloud restore: {error}"))?;
        let snapshot = serde_json::from_str::<UserDataSnapshot>(&content)
            .map_err(|error| format!("failed to parse pending cloud restore: {error}"))?;
        self.apply_user_data_snapshot(&snapshot)?;
        fs::remove_file(path)
            .map_err(|error| format!("failed to finish pending cloud restore: {error}"))
    }

    pub fn restore_user_data_snapshot(
        &self,
        target: &UserDataSnapshot,
        previous: &UserDataSnapshot,
    ) -> Result<(), String> {
        self.write_json("restore.pending.json", target)?;
        if let Err(restore_error) = self.apply_user_data_snapshot(target) {
            self.write_json("restore.pending.json", previous)?;
            match self.apply_user_data_snapshot(previous) {
                Ok(()) => {
                    let _ = fs::remove_file(self.root.join("restore.pending.json"));
                    return Err(format!(
                        "failed to restore cloud data; local data was rolled back: {restore_error}"
                    ));
                }
                Err(rollback_error) => {
                    return Err(format!(
                        "failed to restore cloud data ({restore_error}) and local rollback failed ({rollback_error}); restart CoworkPal to retry the recovery journal"
                    ));
                }
            }
        }
        fs::remove_file(self.root.join("restore.pending.json"))
            .map_err(|error| format!("failed to finish cloud restore: {error}"))
    }

    fn apply_user_data_snapshot(&self, snapshot: &UserDataSnapshot) -> Result<(), String> {
        self.save_settings(&snapshot.settings)?;
        self.save_workshop(&snapshot.workshop)?;
        self.save_layout(&snapshot.layout)?;
        self.save_work_logs(&snapshot.work_logs)?;
        self.save_focus_sessions(&snapshot.focus_sessions)?;
        self.save_achievements(&snapshot.achievements)?;
        self.save_notes(&snapshot.notes)
    }

    pub fn load_or_create_layout(&self) -> Result<LayoutState, String> {
        self.load_or_create("layout.json")
    }

    pub fn save_layout(&self, layout: &LayoutState) -> Result<(), String> {
        self.write_json("layout.json", layout)
    }

    pub fn load_or_create_work_logs(&self) -> Result<WorkLogBook, String> {
        self.load_or_create("work_logs.json")
    }

    pub fn save_work_logs(&self, work_logs: &WorkLogBook) -> Result<(), String> {
        self.write_json("work_logs.json", work_logs)
    }

    pub fn load_or_create_focus_sessions(&self) -> Result<FocusSessionBook, String> {
        self.load_or_create("focus_sessions.json")
    }

    pub fn save_focus_sessions(&self, sessions: &FocusSessionBook) -> Result<(), String> {
        self.write_json("focus_sessions.json", sessions)
    }

    pub fn load_or_create_notes(&self) -> Result<NoteBook, String> {
        self.load_or_create("notes.json")
    }

    pub fn save_notes(&self, notes: &NoteBook) -> Result<(), String> {
        self.write_json("notes.json", notes)
    }

    pub fn load_or_create_achievements(&self) -> Result<AchievementBook, String> {
        self.load_or_create("achievements.json")
    }

    pub fn save_achievements(&self, achievements: &AchievementBook) -> Result<(), String> {
        self.write_json("achievements.json", achievements)
    }

    pub fn take_corruption_rebuilds(&self) -> Vec<String> {
        self.corruption_rebuilds
            .lock()
            .map(|mut rebuilds| std::mem::take(&mut *rebuilds))
            .unwrap_or_default()
    }

    fn load_or_create<T>(&self, file_name: &str) -> Result<T, String>
    where
        T: Default + Serialize + DeserializeOwned,
    {
        let path = self.root.join(file_name);

        if !path.exists() {
            if let Some(value) = self.recover::<T>(file_name)? {
                self.write_json(file_name, &value)?;
                self.cleanup_temp_files(file_name);
                self.record_corruption_rebuild(file_name);
                return Ok(value);
            }
            let value = T::default();
            self.write_json(file_name, &value)?;
            return Ok(value);
        }

        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {file_name}: {error}"))?;

        match serde_json::from_str::<T>(&content) {
            Ok(value) => {
                self.ensure_last_good_backup(file_name, content.as_bytes())?;
                self.cleanup_temp_files(file_name);
                Ok(value)
            }
            Err(error) => {
                if let Err(backup_error) = self.backup_corrupted_file(&path, file_name) {
                    tracing::warn!("could not preserve corrupted {file_name}: {backup_error}");
                }
                match self.recover::<T>(file_name)? {
                    Some(value) => {
                        self.write_json(file_name, &value)?;
                        self.cleanup_temp_files(file_name);
                        self.record_corruption_rebuild(file_name);
                        tracing::warn!("recovered corrupted {file_name}: {error}");
                        Ok(value)
                    }
                    None => Err(format!(
                        "failed to parse {file_name}: {error}; no valid recovery copy was found"
                    )),
                }
            }
        }
    }

    fn write_json<T>(&self, file_name: &str, value: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let path = self.root.join(file_name);
        // Include the PID in the temp file name so two concurrently-running
        // instances don't clobber each other's temp file before the atomic
        // rename (the previous fixed `{file_name}.tmp` let instance B overwrite
        // instance A's temp, then A's rename could commit B's content).
        let temp_path = self
            .root
            .join(format!("{file_name}.{}.tmp", std::process::id()));
        let content = serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize {file_name}: {error}"))?;

        write_synced(&temp_path, content.as_bytes())
            .map_err(|error| format!("failed to write temp {file_name}: {error}"))?;

        // Refresh the recovery copy at most every five minutes. This bounds
        // possible recovery loss without doubling writes for large work logs.
        self.refresh_last_good_backup(file_name, content.as_bytes())?;

        // Atomic replace: a single rename over the existing file. On Windows
        // this uses MoveFileExW with MOVEFILE_REPLACE_EXISTING, so there is no
        // window where the target is absent. The previous remove-then-rename
        // sequence left the file missing between the two calls; a crash there
        // would lose the data file entirely.
        replace_file(&temp_path, &path)
            .map_err(|error| format!("failed to commit {file_name}: {error}"))?;
        Ok(())
    }

    fn ensure_last_good_backup(&self, file_name: &str, content: &[u8]) -> Result<(), String> {
        if self.last_good_path(file_name).exists() {
            return Ok(());
        }
        self.write_last_good_backup(file_name, content)
    }

    fn refresh_last_good_backup(&self, file_name: &str, content: &[u8]) -> Result<(), String> {
        let backup_path = self.last_good_path(file_name);
        let is_fresh = fs::metadata(&backup_path)
            .and_then(|metadata| metadata.modified())
            .and_then(|modified| modified.elapsed().map_err(std::io::Error::other))
            .is_ok_and(|age| age < LAST_GOOD_BACKUP_INTERVAL);
        if is_fresh {
            return Ok(());
        }
        self.write_last_good_backup(file_name, content)
    }

    fn write_last_good_backup(&self, file_name: &str, content: &[u8]) -> Result<(), String> {
        let backup_path = self.last_good_path(file_name);
        let temp_path = self
            .root
            .join("backups")
            .join(format!("{file_name}.{}.backup.tmp", std::process::id()));
        write_synced(&temp_path, content)
            .map_err(|error| format!("failed to write backup for {file_name}: {error}"))?;
        replace_file(&temp_path, &backup_path)
            .map_err(|error| format!("failed to commit backup for {file_name}: {error}"))
    }

    fn last_good_path(&self, file_name: &str) -> PathBuf {
        self.root
            .join("backups")
            .join(format!("{file_name}.last-good.bak"))
    }

    fn recover<T>(&self, file_name: &str) -> Result<Option<T>, String>
    where
        T: DeserializeOwned,
    {
        let mut candidates = self.temp_files(file_name);
        candidates.push(self.last_good_path(file_name));
        let existing: Vec<_> = candidates
            .into_iter()
            .filter(|path| path.exists())
            .collect();

        for candidate in &existing {
            let Ok(content) = fs::read_to_string(candidate) else {
                continue;
            };
            if let Ok(value) = serde_json::from_str::<T>(&content) {
                return Ok(Some(value));
            }
        }

        if existing.is_empty() {
            Ok(None)
        } else {
            Err(format!(
                "all recovery copies for {file_name} are unreadable or invalid"
            ))
        }
    }

    fn temp_files(&self, file_name: &str) -> Vec<PathBuf> {
        let prefix = format!("{file_name}.");
        let mut paths: Vec<_> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".tmp"))
            })
            .collect();
        paths.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
        paths.reverse();
        paths
    }

    fn cleanup_temp_files(&self, file_name: &str) {
        for path in self.temp_files(file_name) {
            let _ = fs::remove_file(path);
        }
    }

    fn backup_corrupted_file(&self, path: &Path, file_name: &str) -> Result<(), String> {
        let backup_name = format!("{file_name}.{}.bak", unix_timestamp_ms());
        let backup_path = self.root.join("backups").join(backup_name);
        fs::copy(path, backup_path)
            .map(|_| ())
            .map_err(|error| format!("failed to backup corrupted {file_name}: {error}"))
    }

    fn record_corruption_rebuild(&self, file_name: &str) {
        if let Ok(mut rebuilds) = self.corruption_rebuilds.lock() {
            rebuilds.push(file_name.to_string());
        }
    }
}

fn write_synced(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    file.sync_all()
}

fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

fn query_smbios_uuid() -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut command = std::process::Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystemProduct).UUID",
        ]);
        command.creation_flags(CREATE_NO_WINDOW);

        // Bounded: a wedged WMI provider at startup must not hang app launch.
        if let Some(output) = crate::process_util::run_command_with_timeout(
            command,
            crate::process_util::DEFAULT_SUBPROCESS_TIMEOUT,
        ) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let uuid = stdout.trim().to_string();
                if !uuid.is_empty()
                    && uuid != "00000000-0000-0000-0000-000000000000"
                    && uuid != "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"
                {
                    return Some(uuid);
                }
            }
        }
    }
    None
}

fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x00000100000001B3u64);
    }
    hash
}

fn convert_uuid_to_cat_id(uuid: &str) -> String {
    let seed = fnv1a_64(uuid.trim().as_bytes());
    let mut state = if seed == 0 { 1234567890 } else { seed };
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(10);

    for _ in 0..10 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let idx = (state % chars.len() as u64) as usize;
        result.push(chars[idx] as char);
    }
    result
}

fn generate_random_cat_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1234567890);

    let mut state = seed as u64;
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(10);

    for _ in 0..10 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let idx = (state % chars.len() as u64) as usize;
        result.push(chars[idx] as char);
    }
    result
}

fn migrate_settings(settings: &mut AppSettings, smbios_uuid: Option<String>) -> bool {
    let mut changed = false;
    if settings.schema_version < APP_SETTINGS_SCHEMA_VERSION {
        if settings.schema_version < 2 {
            settings.is_monitor_bar_visible = false;
            settings.show_monitor_data_in_taskbar = false;
        }
        // v4 replaces the external LibreHardwareMonitor connection with the
        // bundled privileged helper. The new opt-in defaults to false so an
        // upgrade never causes an unexpected UAC prompt.
        if settings.schema_version < 4 {
            settings.integrated_hardware_monitor_enabled = false;
        }
        // v5 lets a user explicitly opt into hardware sensors while minimal
        // mode is active. Clear only the stale contradictory value once when
        // upgrading from versions that could enable the helper accidentally.
        if settings.schema_version < 5 && settings.minimal_mode_enabled {
            settings.integrated_hardware_monitor_enabled = false;
        }
        settings.schema_version = APP_SETTINGS_SCHEMA_VERSION;
        changed = true;
    }

    if settings.cat_id.is_empty() {
        settings.cat_id = smbios_uuid
            .as_deref()
            .map(convert_uuid_to_cat_id)
            .unwrap_or_else(generate_random_cat_id);
        changed = true;
    }

    changed
}

pub(crate) fn app_data_root() -> std::io::Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .map(|root| root.join("CoworkPal"))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "APPDATA/XDG_DATA_HOME is unavailable; refusing to create a temporary save",
            )
        })
}

fn unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("cowork-pal-{name}-{}", unix_timestamp_ms()))
    }

    #[test]
    fn creates_missing_json_files() {
        let root = unique_test_root("create");
        let storage = StorageService::new_with_root(root.clone()).unwrap();

        let settings = storage.load_or_create_settings().unwrap();
        let workshop = storage.load_or_create_workshop().unwrap();
        let layout = storage.load_or_create_layout().unwrap();
        let work_logs = storage.load_or_create_work_logs().unwrap();

        assert_eq!(settings.schema_version, APP_SETTINGS_SCHEMA_VERSION);
        assert!(!settings.is_monitor_bar_visible);
        assert!(!settings.show_monitor_data_in_taskbar);
        assert_eq!(settings.cat_id.len(), 10);
        assert!(settings.cat_id.chars().all(|c| c.is_ascii_alphanumeric()));

        // Verify persistence: loading settings again should return the exact same cat_id
        let settings_again = storage.load_or_create_settings().unwrap();
        assert_eq!(settings_again.cat_id, settings.cat_id);

        assert_eq!(workshop.schema_version, 1);
        assert_eq!(layout.schema_version, 1);
        assert_eq!(work_logs.schema_version, 1);
        assert!(root.join("settings.json").exists());
        assert!(root.join("save.json").exists());
        assert!(root.join("layout.json").exists());
        assert!(root.join("work_logs.json").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovers_corrupted_json_from_last_good_backup() {
        let root = unique_test_root("corrupt");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let workshop = WorkshopState {
            workshop_level: 18,
            ..WorkshopState::default()
        };
        storage.save_workshop(&workshop).unwrap();
        fs::write(root.join("save.json"), "{not-json").unwrap();

        let recovered = storage.load_or_create_workshop().unwrap();

        assert_eq!(recovered.workshop_level, 18);
        let backup_count = fs::read_dir(root.join("backups")).unwrap().count();
        assert_eq!(backup_count, 2);
        assert_eq!(
            storage.take_corruption_rebuilds(),
            vec!["save.json".to_string()]
        );
        assert!(storage.take_corruption_rebuilds().is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn corrupted_json_without_recovery_is_not_replaced_with_default() {
        let root = unique_test_root("corrupt-no-recovery");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        fs::write(root.join("save.json"), "{not-json").unwrap();

        let error = storage.load_or_create_workshop().unwrap_err();

        assert!(error.contains("no valid recovery copy"));
        assert_eq!(
            fs::read_to_string(root.join("save.json")).unwrap(),
            "{not-json"
        );
        assert!(storage.take_corruption_rebuilds().is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovers_missing_primary_from_complete_temp_file() {
        let root = unique_test_root("temp-recovery");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let workshop = WorkshopState {
            workshop_level: 23,
            ..WorkshopState::default()
        };
        fs::write(
            root.join("save.json.999999.tmp"),
            serde_json::to_string_pretty(&workshop).unwrap(),
        )
        .unwrap();

        let recovered = storage.load_or_create_workshop().unwrap();

        assert_eq!(recovered.workshop_level, 23);
        assert!(root.join("save.json").exists());
        assert!(!root.join("save.json.999999.tmp").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repeated_save_replaces_primary_and_keeps_recent_backup() {
        let root = unique_test_root("replace");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let mut workshop = storage.load_or_create_workshop().unwrap();
        workshop.workshop_level = 31;

        storage.save_workshop(&workshop).unwrap();

        assert_eq!(
            storage.load_or_create_workshop().unwrap().workshop_level,
            31
        );
        let backup: WorkshopState =
            serde_json::from_str(&fs::read_to_string(storage.last_good_path("save.json")).unwrap())
                .unwrap();
        assert_eq!(backup.workshop_level, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_preserves_cat_id_workshop_level_and_work_history() {
        let root = unique_test_root("restart");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let settings = storage.load_or_create_settings().unwrap();
        let workshop = WorkshopState {
            workshop_level: 42,
            ..WorkshopState::default()
        };
        storage.save_workshop(&workshop).unwrap();
        let mut work_logs = WorkLogBook::default();
        work_logs.entries.insert(
            "2026-08-25".to_string(),
            crate::models::WorkLogEntry {
                date: "2026-08-25".to_string(),
                active_seconds: 3_600,
                ..crate::models::WorkLogEntry::default()
            },
        );
        storage.save_work_logs(&work_logs).unwrap();
        drop(storage);

        let restarted = StorageService::new_with_root(root.clone()).unwrap();
        let settings_after = restarted.load_or_create_settings().unwrap();
        let workshop_after = restarted.load_or_create_workshop().unwrap();
        let logs_after = restarted.load_or_create_work_logs().unwrap();

        assert_eq!(settings_after.cat_id, settings.cat_id);
        assert_eq!(workshop_after.workshop_level, 42);
        assert_eq!(logs_after.entries["2026-08-25"].active_seconds, 3_600);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migration_repairs_legacy_minimal_mode_hardware_monitor_setting() {
        let root = unique_test_root("minimal-repair");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let settings = AppSettings {
            schema_version: 4,
            minimal_mode_enabled: true,
            is_cat_visible: true,
            is_monitor_bar_visible: true,
            enable_low_power_mode: false,
            integrated_hardware_monitor_enabled: true,
            ..AppSettings::default()
        };
        storage.save_settings(&settings).unwrap();

        let repaired = storage.load_or_create_settings().unwrap();

        assert!(!repaired.is_cat_visible);
        assert!(!repaired.is_monitor_bar_visible);
        assert!(repaired.enable_low_power_mode);
        assert!(!repaired.integrated_hardware_monitor_enabled);
        let persisted: AppSettings =
            serde_json::from_str(&fs::read_to_string(root.join("settings.json")).unwrap()).unwrap();
        assert!(!persisted.integrated_hardware_monitor_enabled);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn current_minimal_mode_preserves_explicit_hardware_monitor_opt_in() {
        let root = unique_test_root("minimal-monitor-opt-in");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let settings = AppSettings {
            minimal_mode_enabled: true,
            is_cat_visible: false,
            is_monitor_bar_visible: false,
            enable_low_power_mode: true,
            integrated_hardware_monitor_enabled: true,
            ..AppSettings::default()
        };
        storage.save_settings(&settings).unwrap();

        let loaded = storage.load_or_create_settings().unwrap();

        assert!(loaded.minimal_mode_enabled);
        assert!(loaded.integrated_hardware_monitor_enabled);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migrates_monitor_surfaces_to_lazy_startup() {
        let root = unique_test_root("settings-migrate");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let mut settings = AppSettings::default();
        settings.schema_version = 1;
        settings.is_monitor_bar_visible = true;
        settings.show_monitor_data_in_taskbar = true;
        storage.save_settings(&settings).unwrap();

        let migrated = storage.load_or_create_settings().unwrap();

        assert_eq!(migrated.schema_version, APP_SETTINGS_SCHEMA_VERSION);
        assert!(!migrated.is_monitor_bar_visible);
        assert!(!migrated.show_monitor_data_in_taskbar);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migration_does_not_carry_external_monitor_opt_in_to_privileged_helper() {
        let root = unique_test_root("integrated-monitor-migrate");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        fs::write(
            root.join("settings.json"),
            r#"{"schemaVersion":3,"libreHardwareMonitorEnabled":true}"#,
        )
        .unwrap();

        let migrated = storage.load_or_create_settings().unwrap();

        assert_eq!(migrated.schema_version, APP_SETTINGS_SCHEMA_VERSION);
        assert!(!migrated.integrated_hardware_monitor_enabled);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_uuid_cat_id_conversion() {
        let uuid1 = "232BAA30-C234-5440-95F2-E3028F9818EC";
        let uuid2 = "232BAA30-C234-5440-95F2-E3028F9818EC "; // with trailing space
        let uuid3 = "9F52E302-8F98-18EC-232B-AA30C2345440"; // different UUID

        let id1 = convert_uuid_to_cat_id(uuid1);
        let id2 = convert_uuid_to_cat_id(uuid2);
        let id3 = convert_uuid_to_cat_id(uuid3);

        assert_eq!(id1.len(), 10);
        assert!(id1.chars().all(|c| c.is_ascii_alphanumeric()));

        // Test determinism (same input, same output)
        assert_eq!(id1, id2);

        // Test distinctness (different input, different output)
        assert_ne!(id1, id3);
    }

    #[test]
    fn existing_cat_id_does_not_change_with_smbios_uuid() {
        let mut settings = AppSettings {
            schema_version: APP_SETTINGS_SCHEMA_VERSION,
            cat_id: "Existing01".to_string(),
            ..AppSettings::default()
        };

        let changed = migrate_settings(
            &mut settings,
            Some("232BAA30-C234-5440-95F2-E3028F9818EC".to_string()),
        );

        assert!(!changed);
        assert_eq!(settings.cat_id, "Existing01");
    }

    #[test]
    fn cloud_restore_persists_all_user_data_and_clears_journal() {
        let root = unique_test_root("cloud-restore");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        let previous = UserDataSnapshot {
            schema_version: 1,
            exported_at: 1,
            app_version: "test".to_string(),
            device_profile: Default::default(),
            settings: AppSettings::default(),
            workshop: WorkshopState::default(),
            layout: LayoutState::default(),
            work_logs: WorkLogBook::default(),
            focus_sessions: FocusSessionBook::default(),
            achievements: AchievementBook::default(),
            notes: NoteBook::default(),
        };
        let mut target = previous.clone();
        target.workshop.workshop_level = 27;
        target.notes.notes.push(crate::models::Note {
            id: "note-cloud".to_string(),
            title: "云端笔记".to_string(),
            ..crate::models::Note::default()
        });

        storage
            .restore_user_data_snapshot(&target, &previous)
            .unwrap();

        assert_eq!(
            storage.load_or_create_workshop().unwrap().workshop_level,
            27
        );
        assert_eq!(
            storage.load_or_create_notes().unwrap().notes[0].id,
            "note-cloud"
        );
        assert!(!root.join("restore.pending.json").exists());

        let _ = fs::remove_dir_all(root);
    }
}
