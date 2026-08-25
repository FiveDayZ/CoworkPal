use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;

use crate::{
    achievements::{compact_achievement_book, AchievementBook},
    memory_release::MemoryReleaseState,
    models::{
        AppSettings, CatRuntimeState, FocusSessionBook, HardwareSnapshot, LayoutState, NoteBook,
        WorkLogBook, WorkshopState,
    },
    monitoring::{create_default_adapter, HardwareSensorAdapter},
    storage::StorageService,
};

pub struct AppState {
    pub settings: RwLock<AppSettings>,
    pub workshop: RwLock<WorkshopState>,
    pub layout: RwLock<LayoutState>,
    pub work_logs: RwLock<WorkLogBook>,
    pub focus_sessions: RwLock<FocusSessionBook>,
    pub achievements: RwLock<AchievementBook>,
    pub notes: RwLock<NoteBook>,
    pub last_snapshot: RwLock<Option<HardwareSnapshot>>,
    /// All CoCat runtime fields behind one lock so state updates are atomic.
    pub cat_runtime: RwLock<CatRuntimeState>,
    pub storage: StorageService,
    /// `Arc` so the adapter can be cloned into `spawn_blocking` closures,
    /// keeping subprocess work (nvidia-smi / powershell) off the async runtime.
    pub hardware_adapter: Arc<Mutex<Box<dyn HardwareSensorAdapter>>>,
    /// Memory release watcher cooldown state (auto-trigger only).
    pub memory_release: MemoryReleaseState,
}

impl AppState {
    pub fn load() -> Result<Self, String> {
        let storage = StorageService::new().map_err(|error| error.to_string())?;
        storage.apply_pending_user_data_restore()?;

        // Never put a default value into live state when an existing data file
        // merely failed to load. Background pumps persist these values, so doing
        // that would turn a transient read error into permanent data loss.
        let settings = storage
            .load_or_create_settings()
            .map_err(|error| format!("settings.json: {error}"))?;
        let workshop = storage
            .load_or_create_workshop()
            .map_err(|error| format!("save.json: {error}"))?;
        let layout = storage
            .load_or_create_layout()
            .map_err(|error| format!("layout.json: {error}"))?;
        let work_logs = storage
            .load_or_create_work_logs()
            .map_err(|error| format!("work_logs.json: {error}"))?;
        let focus_sessions = storage
            .load_or_create_focus_sessions()
            .map_err(|error| format!("focus_sessions.json: {error}"))?;
        let mut achievements = storage
            .load_or_create_achievements()
            .map_err(|error| format!("achievements.json: {error}"))?;
        if compact_achievement_book(&mut achievements) {
            if let Err(error) = storage.save_achievements(&achievements) {
                tracing::warn!("failed to persist compacted achievement history: {error}");
            }
        }
        let notes = storage
            .load_or_create_notes()
            .map_err(|error| format!("notes.json: {error}"))?;

        Ok(Self {
            settings: RwLock::new(settings),
            workshop: RwLock::new(workshop),
            layout: RwLock::new(layout),
            work_logs: RwLock::new(work_logs),
            focus_sessions: RwLock::new(focus_sessions),
            achievements: RwLock::new(achievements),
            notes: RwLock::new(notes),
            last_snapshot: RwLock::new(None),
            cat_runtime: RwLock::new(CatRuntimeState::default()),
            storage,
            hardware_adapter: Arc::new(Mutex::new(create_default_adapter())),
            memory_release: MemoryReleaseState::default(),
        })
    }
}
