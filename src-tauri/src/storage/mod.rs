use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    achievements::AchievementBook,
    models::{
        AppSettings, FocusSessionBook, LayoutState, NoteBook, WorkLogBook, WorkshopState,
        APP_SETTINGS_SCHEMA_VERSION,
    },
};

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
        let root = app_data_root();
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
        if migrate_settings(&mut settings, self.cached_smbios_uuid()) {
            self.save_settings(&settings)?;
        }
        Ok(settings)
    }

    /// Returns the cached SMBIOS UUID, querying powershell only on the first call.
    fn cached_smbios_uuid(&self) -> Option<String> {
        self.smbios_uuid
            .get_or_init(|| query_smbios_uuid())
            .clone()
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
            let value = T::default();
            self.write_json(file_name, &value)?;
            return Ok(value);
        }

        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {file_name}: {error}"))?;

        match serde_json::from_str::<T>(&content) {
            Ok(value) => {
                self.write_json(file_name, &value)?;
                Ok(value)
            }
            Err(error) => {
                self.backup_corrupted_file(&path, file_name)?;
                let value = T::default();
                self.write_json(file_name, &value)?;
                self.record_corruption_rebuild(file_name);
                tracing::warn!("rebuilt corrupted {file_name}: {error}");
                Ok(value)
            }
        }
    }

    fn write_json<T>(&self, file_name: &str, value: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let path = self.root.join(file_name);
        let temp_path = self.root.join(format!("{file_name}.tmp"));
        let content = serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize {file_name}: {error}"))?;

        fs::write(&temp_path, content)
            .map_err(|error| format!("failed to write temp {file_name}: {error}"))?;

        // Atomic replace: a single rename over the existing file. On Windows
        // this uses MoveFileExW with MOVEFILE_REPLACE_EXISTING, so there is no
        // window where the target is absent. The previous remove-then-rename
        // sequence left the file missing between the two calls; a crash there
        // would lose the data file entirely.
        fs::rename(&temp_path, &path)
            .map_err(|error| format!("failed to commit {file_name}: {error}"))
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

        if let Ok(output) = command.output() {
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
        // v3 added `libre_hardware_monitor_enabled`. Its value is supplied by
        // serde's `#[serde(default)]` (true) during deserialization of older
        // files, so no explicit assignment is needed here.
        settings.schema_version = APP_SETTINGS_SCHEMA_VERSION;
        changed = true;
    }

    let target_cat_id = if let Some(uuid) = smbios_uuid {
        convert_uuid_to_cat_id(&uuid)
    } else {
        if settings.cat_id.is_empty() {
            generate_random_cat_id()
        } else {
            settings.cat_id.clone()
        }
    };

    if settings.cat_id != target_cat_id {
        settings.cat_id = target_cat_id;
        changed = true;
    }

    changed
}

fn app_data_root() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("CoreWorkPal")
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
        std::env::temp_dir().join(format!("core-work-pal-{name}-{}", unix_timestamp_ms()))
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
    fn backs_up_corrupted_json_and_rebuilds_default() {
        let root = unique_test_root("corrupt");
        let storage = StorageService::new_with_root(root.clone()).unwrap();
        fs::write(root.join("settings.json"), "{not-json").unwrap();

        let settings = storage.load_or_create_settings().unwrap();

        assert_eq!(settings.schema_version, APP_SETTINGS_SCHEMA_VERSION);
        let backup_count = fs::read_dir(root.join("backups")).unwrap().count();
        assert_eq!(backup_count, 1);
        assert_eq!(
            storage.take_corruption_rebuilds(),
            vec!["settings.json".to_string()]
        );
        assert!(storage.take_corruption_rebuilds().is_empty());

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
}
