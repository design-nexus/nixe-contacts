pub use nixe_theme::ThemeMode;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub mode: ThemeMode,
    pub theme: String,
    pub glow: bool,
    pub reduce_motion: bool,
    pub sidebar_collapsed: bool,
    pub last_section: String,
    pub last_first: bool,
    pub sort_last: bool,
}
impl Default for Prefs {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Omarchy,
            theme: "tokyo-night".into(),
            glow: true,
            reduce_motion: false,
            sidebar_collapsed: false,
            last_section: "all".into(),
            last_first: false,
            sort_last: false,
        }
    }
}
nixe_core::prefs_store!(Prefs);
