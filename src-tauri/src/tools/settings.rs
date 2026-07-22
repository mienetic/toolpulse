//! User settings persisted as JSON in the OS config directory.
//!
//! Defaults enable every builtin tool and turn auto-check on at startup. The
//! frontend reads/writes this through the `get_settings`/`save_settings`
//! commands.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::registry::builtin_tools;
use super::types::*;

/// How the background scheduler decides when to fire a notification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotifyMode {
    /// Fire every `notify_interval_hours`, regardless of count.
    Interval,
    /// Spread `notify_max_per_day` notifications evenly across the day.
    DailyCount,
    /// Interval cadence, but never more than `notify_max_per_day` in 24h.
    Both,
    /// Notifications disabled entirely.
    Off,
}

impl Default for NotifyMode {
    fn default() -> Self {
        NotifyMode::Both
    }
}

/// Notification scheduling + quiet-hours configuration.
///
/// All times are local hours on a 24h clock (0–23). Quiet hours wrap across
/// midnight when `end < start` (e.g. start=22, end=8 means 22:00–08:00).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Scheduling strategy.
    #[serde(default)]
    pub mode: NotifyMode,
    /// Hours between notifications when mode is `Interval` or `Both`.
    #[serde(default = "default_interval_hours")]
    pub interval_hours: u32,
    /// Hard cap on notifications per rolling 24h window.
    #[serde(default = "default_max_per_day")]
    pub max_per_day: u32,
    /// Only notify when at least one tool is outdated.
    #[serde(default = "default_true")]
    pub only_updates: bool,
    /// Skip a tool that was already notified about earlier today.
    #[serde(default = "default_true")]
    pub dedupe_same_day: bool,
    /// Start of the do-not-disturb window (inclusive), 0–23.
    #[serde(default = "default_quiet_start")]
    pub quiet_hours_start: u32,
    /// End of the do-not-disturb window (exclusive), 0–23.
    #[serde(default = "default_quiet_end")]
    pub quiet_hours_end: u32,
}

fn default_interval_hours() -> u32 {
    6
}
fn default_max_per_day() -> u32 {
    4
}
fn default_quiet_start() -> u32 {
    22
}
fn default_quiet_end() -> u32 {
    8
}
fn default_true() -> bool {
    true
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            mode: NotifyMode::default(),
            interval_hours: default_interval_hours(),
            max_per_day: default_max_per_day(),
            only_updates: true,
            dedupe_same_day: true,
            quiet_hours_start: default_quiet_start(),
            quiet_hours_end: default_quiet_end(),
        }
    }
}

impl NotificationSettings {
    /// True if the given local hour falls inside the quiet window.
    ///
    /// Handles wrap-around: start=22, end=8 covers 22, 23, 0..=7.
    pub fn is_quiet_hour(&self, hour: u32) -> bool {
        if self.quiet_hours_start == self.quiet_hours_end {
            return false;
        }
        if self.quiet_hours_start < self.quiet_hours_end {
            hour >= self.quiet_hours_start && hour < self.quiet_hours_end
        } else {
            // Wraps midnight.
            hour >= self.quiet_hours_start || hour < self.quiet_hours_end
        }
    }

    /// The effective cadence the scheduler should poll at, in seconds.
    ///
    /// We poll frequently and let the gating logic decide whether to actually
    /// fire; this keeps the scheduler simple and responsive to setting changes.
    pub fn poll_interval_secs(&self) -> u64 {
        match self.mode {
            NotifyMode::Off => 3600, // idle poll; harmless
            NotifyMode::Interval | NotifyMode::Both => {
                // Wake up 4x per interval window so we don't overshoot.
                (self.interval_hours as u64 * 3600).max(900).min(3600 * 6)
            }
            NotifyMode::DailyCount => {
                // Check roughly hourly so the daily budget spreads evenly.
                3600
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Builtin tool names that should appear in scans.
    #[serde(default = "default_enabled_tools")]
    pub enabled_tools: Vec<String>,
    /// Whether to run a scan automatically when the app starts.
    #[serde(default = "default_true")]
    pub auto_check_on_start: bool,
    /// Minimum hours between automatic re-checks.
    #[serde(default = "default_interval")]
    pub auto_check_interval_hours: u32,
    /// `"dark"` or `"light"`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Whether the floating overlay window is shown.
    #[serde(default = "default_true")]
    pub overlay_enabled: bool,
    /// Notification + scheduling preferences.
    #[serde(default)]
    pub notifications: NotificationSettings,
    /// User-chosen default installation path per tool.
    ///
    /// When a tool has multiple installations, the user can pin one as the
    /// "tracked" version. The key is the tool name (e.g. `"node"`), the value
    /// is the absolute binary path. Absent entries fall back to the active
    /// PATH entry.
    #[serde(default)]
    pub selected_paths: std::collections::HashMap<String, String>,
}

fn default_enabled_tools() -> Vec<String> {
    builtin_tools().into_iter().map(|t| t.name).collect()
}
fn default_interval() -> u32 {
    24
}
fn default_theme() -> String {
    "dark".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_tools: default_enabled_tools(),
            auto_check_on_start: true,
            auto_check_interval_hours: default_interval(),
            theme: default_theme(),
            overlay_enabled: true,
            notifications: NotificationSettings::default(),
            selected_paths: std::collections::HashMap::new(),
        }
    }
}

/// Load settings from `path`, falling back to defaults on any error.
pub fn load(path: &PathBuf) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Persist settings to `path`, creating parent directories as needed.
pub fn save(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, body).map_err(|e| e.to_string())
}

/// Resolve the active list of tool definitions based on `enabled_tools`.
///
/// Disabled tools are dropped from the scan entirely. Unknown names are
/// ignored so stale config doesn't crash the app.
pub fn active_definitions(settings: &Settings) -> Vec<ToolDefinition> {
    builtin_tools()
        .into_iter()
        .filter(|t| settings.enabled_tools.iter().any(|n| n == &t.name))
        .collect()
}
