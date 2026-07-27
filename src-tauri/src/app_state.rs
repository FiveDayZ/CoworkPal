use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;

use crate::{
    achievements::AchievementBook,
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
    /// All CoreCat runtime fields behind one lock so state updates are atomic.
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
        let settings = storage.load_or_create_settings()?;
        let workshop = storage.load_or_create_workshop()?;
        let layout = storage.load_or_create_layout()?;
        let work_logs = storage.load_or_create_work_logs()?;
        let focus_sessions = storage.load_or_create_focus_sessions()?;
        let achievements = storage.load_or_create_achievements()?;
        let notes = storage.load_or_create_notes()?;

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
