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

        // Each data file is loaded through `or_default` so a single non-critical
        // file that cannot be read/written (permissions, locked, disk error)
        // degrades to a fresh default instead of aborting app launch entirely.
        // `StorageService::new()` (which creates the data dir) is still fatal —
        // without a writable root nothing can persist.
        //
        // JSON-corruption is already handled inside load_or_create (it backs up
        // and rebuilds); the fallthrough here covers the remaining IO errors.
        //
        // Caveat for settings specifically: if settings.json fails to load due
        // to a transient IO error (file intact but momentarily unreadable),
        // falling back to defaults means the next save_settings will overwrite
        // the still-intact file with defaults, permanently losing the user's
        // config. We accept this trade-off (app launching > config preserved)
        // because JSON corruption — the common failure — is already handled
        // safely upstream, and a pure IO read failure of an intact file is rare.
        let settings = load_or_default(&storage, StorageService::load_or_create_settings);
        let workshop = load_or_default(&storage, StorageService::load_or_create_workshop);
        let layout = load_or_default(&storage, StorageService::load_or_create_layout);
        let work_logs = load_or_default(&storage, StorageService::load_or_create_work_logs);
        let focus_sessions =
            load_or_default(&storage, StorageService::load_or_create_focus_sessions);
        let achievements =
            load_or_default(&storage, StorageService::load_or_create_achievements);
        let notes = load_or_default(&storage, StorageService::load_or_create_notes);

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

/// Load a data file, falling back to its default on any error.
///
/// A failure is logged loudly but never aborts startup: losing one file's
/// state (e.g. achievements.json unreadable) must not make the whole app
/// unlaunchable. The user still gets a working app with that dataset reset,
/// rather than a crash on launch they cannot recover from.
fn load_or_default<T: Default, F>(storage: &StorageService, loader: F) -> T
where
    F: FnOnce(&StorageService) -> Result<T, String>,
{
    match loader(storage) {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(
                "failed to load a data file, continuing with defaults: {error}"
            );
            T::default()
        }
    }
}
