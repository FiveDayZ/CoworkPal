//! Pure-local process-name → category classifier.
//!
//! Used to pick a "story lead" process so CoreCat's speech-bubble text can
//! reference what the user is actually doing (compiling, browsing, …) without
//! adding new `CatState` variants. The full process list stays local; only the
//! chosen category name influences the bubble text.
//!
//! Matching is lowercase substring against a curated table. It is deliberately
//! conservative: unknown processes fall back to `Unknown` and the caller keeps
//! the original static bubble text.

use crate::models::ProcessUsageSnapshot;

/// Coarse classification of a foreground-heavy process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessCategory {
    /// Build toolchain / compiler / bundler (node, cargo, msbuild, …).
    Compiler,
    /// Web browser.
    Browser,
    /// Code editor or IDE.
    Ide,
    /// Video conferencing / chat client.
    VideoCall,
    /// Media playback.
    Media,
    /// PC game / game launcher.
    Game,
    /// Anything not in the table — caller should keep the default text.
    Unknown,
}

impl ProcessCategory {
    pub fn is_story_worthy(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// (needle, category) pairs scanned in order. First substring hit (case-folded)
/// wins. Game needles intentionally sit above Browser/Ide so a process like
/// `steamwebhelper.exe` still classifies as a game when steam is the lead.
///
/// Needles are matched against the normalized (lowercased, `.exe`/`.bat`
/// stripped) name, so they are written WITHOUT an extension. Short needles
/// that could false-positive as substrings of unrelated words are either
/// lengthened to a distinguishing fragment or matched exactly.
const RULES: &[(&str, ProcessCategory)] = &[
    // --- Games / launchers (checked first) ---
    ("steam", ProcessCategory::Game),
    ("epicgameslauncher", ProcessCategory::Game),
    ("galaxyclient", ProcessCategory::Game), // GOG Galaxy
    ("battle.net", ProcessCategory::Game),
    ("riotclient", ProcessCategory::Game),
    ("league of legends", ProcessCategory::Game),
    ("dota", ProcessCategory::Game),
    ("genshinimpact", ProcessCategory::Game),
    ("mihoyo", ProcessCategory::Game),
    ("unity", ProcessCategory::Game),
    ("unreal", ProcessCategory::Game),
    ("javaw", ProcessCategory::Game), // Minecraft / most Java games
    // --- Compilers / build toolchains ---
    ("msbuild", ProcessCategory::Compiler),
    ("dotnet", ProcessCategory::Compiler),
    ("roslyn", ProcessCategory::Compiler),
    ("rustc", ProcessCategory::Compiler),
    ("cargo", ProcessCategory::Compiler),
    ("clang", ProcessCategory::Compiler),
    ("ld-link", ProcessCategory::Compiler),
    ("node", ProcessCategory::Compiler),
    ("npm", ProcessCategory::Compiler),
    ("pnpm", ProcessCategory::Compiler),
    ("yarn", ProcessCategory::Compiler),
    ("esbuild", ProcessCategory::Compiler),
    ("vite", ProcessCategory::Compiler),
    ("webpack", ProcessCategory::Compiler),
    ("rollup", ProcessCategory::Compiler),
    ("bazel", ProcessCategory::Compiler),
    ("cmake", ProcessCategory::Compiler),
    ("mingw32-make", ProcessCategory::Compiler),
    ("java", ProcessCategory::Compiler), // often a build/server process
    ("gradle", ProcessCategory::Compiler),
    ("maven", ProcessCategory::Compiler),
    ("docker", ProcessCategory::Compiler),
    // --- Browsers ---
    ("chrome", ProcessCategory::Browser),
    ("msedge", ProcessCategory::Browser),
    ("firefox", ProcessCategory::Browser),
    ("brave", ProcessCategory::Browser),
    ("opera", ProcessCategory::Browser),
    ("vivaldi", ProcessCategory::Browser),
    // --- IDEs / editors ---
    ("code", ProcessCategory::Ide),       // VS Code / code.exe
    ("code-insiders", ProcessCategory::Ide),
    ("cursor", ProcessCategory::Ide),
    ("devenv", ProcessCategory::Ide),     // Visual Studio
    ("idea", ProcessCategory::Ide),       // IntelliJ idea64.exe
    ("webstorm", ProcessCategory::Ide),
    ("pycharm", ProcessCategory::Ide),
    ("clion", ProcessCategory::Ide),
    ("goland", ProcessCategory::Ide),
    ("rustrover", ProcessCategory::Ide),
    ("rider", ProcessCategory::Ide),
    ("sublime_text", ProcessCategory::Ide),
    ("emacs", ProcessCategory::Ide),
    ("atom", ProcessCategory::Ide),
    // --- Video calls / chat ---
    ("zoom", ProcessCategory::VideoCall),
    ("teams", ProcessCategory::VideoCall),
    ("slack", ProcessCategory::VideoCall),
    ("discord", ProcessCategory::VideoCall),
    ("wechat", ProcessCategory::VideoCall),
    ("dingtalk", ProcessCategory::VideoCall),
    ("feishu", ProcessCategory::VideoCall),
    ("lark", ProcessCategory::VideoCall),
    ("skype", ProcessCategory::VideoCall),
    ("tencentmeeting", ProcessCategory::VideoCall),
    // --- Media ---
    ("vlc", ProcessCategory::Media),
    ("spotify", ProcessCategory::Media),
    ("foobar2000", ProcessCategory::Media),
    ("netease_cloudmusic", ProcessCategory::Media),
    ("cloudmusic", ProcessCategory::Media),
    ("qqmusic", ProcessCategory::Media),
    ("kugou", ProcessCategory::Media),
    ("potplayer", ProcessCategory::Media),
];

/// Short names that would false-positive as substrings of unrelated words
/// (e.g. `"go"` in `"googledrive"`, `"link"` in `"hyperlink"`, `"vim"` in
/// `"vimto"`). Matched against the normalized name by exact equality only.
const EXACT_RULES: &[(&str, ProcessCategory)] = &[
    ("go", ProcessCategory::Compiler),
    ("gcc", ProcessCategory::Compiler),
    ("csc", ProcessCategory::Compiler),
    ("vbc", ProcessCategory::Compiler),
    ("fsc", ProcessCategory::Compiler),
    ("swc", ProcessCategory::Compiler),
    ("tsc", ProcessCategory::Compiler),
    ("link", ProcessCategory::Compiler),
    ("make", ProcessCategory::Compiler),
    ("vim", ProcessCategory::Ide),
    ("gvim", ProcessCategory::Ide),
    ("nvim", ProcessCategory::Ide),
    ("arc", ProcessCategory::Browser),
    ("zed", ProcessCategory::Ide),
    ("mpv", ProcessCategory::Media),
];

/// Strip an `.exe` suffix and lowercase for matching.
fn normalize(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let trimmed = lower.trim();
    trimmed
        .strip_suffix(".exe")
        .or_else(|| trimmed.strip_suffix(".bat"))
        .unwrap_or(trimmed)
        .to_string()
}

/// Classify a process by its executable name. Never panics, never allocates
/// beyond the lowercased name. Pure function, safe to call every tick.
pub fn classify_process(name: &str) -> ProcessCategory {
    if name.is_empty() {
        return ProcessCategory::Unknown;
    }
    let key = normalize(name);
    // Exact matches first — short names that would collide as substrings.
    for (needle, category) in EXACT_RULES {
        if key == *needle {
            return *category;
        }
    }
    for (needle, category) in RULES {
        if key.contains(needle) {
            return *category;
        }
    }
    ProcessCategory::Unknown
}

/// Pick the highest-scored process from an already-sorted snapshot list that
/// falls into a story-worthy category. Returns `None` when no candidate is
/// recognized, so the caller keeps the original static bubble text.
///
/// The snapshot list produced by the adapter is sorted by
/// `process_snapshot_score` (CPU-weighted), so the first story-worthy entry is
/// the natural "lead".
pub fn pick_story_lead(processes: &[ProcessUsageSnapshot]) -> Option<&ProcessUsageSnapshot> {
    processes
        .iter()
        .find(|process| classify_process(&process.name).is_story_worthy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_compilers() {
        assert_eq!(classify_process("node.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("MSBuild.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("cargo"), ProcessCategory::Compiler);
        assert_eq!(classify_process("rustc.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("clang-16.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("vite.exe"), ProcessCategory::Compiler);
    }

    #[test]
    fn classifies_browsers_and_games_with_game_priority() {
        // Plain browser.
        assert_eq!(classify_process("chrome.exe"), ProcessCategory::Browser);
        // steamwebhelper still classifies as a Game (game rule sits above browser).
        assert_eq!(
            classify_process("steamwebhelper.exe"),
            ProcessCategory::Game
        );
        assert_eq!(classify_process("steam.exe"), ProcessCategory::Game);
    }

    #[test]
    fn classifies_ide_and_calls_and_media() {
        assert_eq!(classify_process("Code.exe"), ProcessCategory::Ide);
        assert_eq!(classify_process("idea64.exe"), ProcessCategory::Ide);
        assert_eq!(classify_process("cursor.exe"), ProcessCategory::Ide);
        assert_eq!(classify_process("Zoom.exe"), ProcessCategory::VideoCall);
        assert_eq!(classify_process("Spotify.exe"), ProcessCategory::Media);
    }

    #[test]
    fn exact_short_needles_match_their_name() {
        // These short names are matched by exact equality (see EXACT_RULES).
        assert_eq!(classify_process("go.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("go"), ProcessCategory::Compiler);
        assert_eq!(classify_process("make.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("link.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("tsc.exe"), ProcessCategory::Compiler);
        assert_eq!(classify_process("vim.exe"), ProcessCategory::Ide);
        assert_eq!(classify_process("mpv.exe"), ProcessCategory::Media);
        assert_eq!(classify_process("arc.exe"), ProcessCategory::Browser);
    }

    #[test]
    fn exact_short_needles_do_not_false_positive_as_substrings() {
        // `go` must NOT match `googledrive` / `mongo`-style names.
        assert_eq!(
            classify_process("GoogleDriveFS.exe"),
            ProcessCategory::Unknown
        );
        // `link` must NOT match a name that merely ends in "link".
        assert_eq!(
            classify_process("crosslink-helper.exe"),
            ProcessCategory::Unknown
        );
        // `make` must NOT match imagemagick's `magick.exe`.
        assert_eq!(classify_process("magick.exe"), ProcessCategory::Unknown);
        // `vim` must NOT match names that merely contain those letters.
        assert_eq!(classify_process("admvimeo.exe"), ProcessCategory::Unknown);
    }

    #[test]
    fn mingw_make_still_classified() {
        // Substring rule `mingw32-make` must fire before a hypothetical exact.
        assert_eq!(
            classify_process("mingw32-make.exe"),
            ProcessCategory::Compiler
        );
    }

    #[test]
    fn unknown_for_unrecognized_or_empty() {
        assert_eq!(classify_process("some-random-app.exe"), ProcessCategory::Unknown);
        assert_eq!(classify_process(""), ProcessCategory::Unknown);
        assert_eq!(classify_process("explorer.exe"), ProcessCategory::Unknown);
    }

    #[test]
    fn pick_story_lead_skips_unknown_and_takes_first_story_worthy() {
        // Ordered as the adapter emits (already sorted by score).
        let processes = vec![
            snapshot("explorer.exe", 1.0), // System-ish, Unknown → skipped
            snapshot("node.exe", 58.0),    // Compiler → lead
            snapshot("chrome.exe", 5.0),
        ];
        let lead = pick_story_lead(&processes).expect("expected a lead");
        assert_eq!(lead.name, "node.exe");
        assert_eq!(classify_process(&lead.name), ProcessCategory::Compiler);
    }

    #[test]
    fn pick_story_lead_returns_none_when_all_unknown() {
        let processes = vec![snapshot("explorer.exe", 1.0), snapshot("foo bar", 0.4)];
        assert!(pick_story_lead(&processes).is_none());
    }

    fn snapshot(name: &str, cpu: f32) -> ProcessUsageSnapshot {
        ProcessUsageSnapshot {
            pid: 1,
            name: name.to_string(),
            cpu_usage_percent: cpu,
            memory_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_write_bytes_per_second: None,
        }
    }
}
