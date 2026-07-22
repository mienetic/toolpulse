//! Shared application state held in Tauri's managed-state container.
//!
//! A single `AppState` is constructed at startup and cloned cheaply into each
//! command handler (the expensive pieces — HTTP client, DB handle — live
//! behind `Arc`/`Mutex`).

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::tools::{History, LatestCache};

pub struct AppState {
    /// Shared HTTP client with a sensible default timeout and redirect policy.
    pub http: reqwest::Client,
    /// In-memory TTL cache for latest-version lookups.
    pub latest_cache: Arc<LatestCache>,
    /// SQLite history DB. `None` if opening the DB failed at startup.
    pub history: Mutex<Option<History>>,
    /// Absolute path to the JSON settings file.
    pub settings_path: PathBuf,
    /// Mutable state for the notification scheduler (counts, dedupe set).
    pub scheduler: tokio::sync::Mutex<crate::scheduler_state::SchedulerState>,
    /// Tracks in-flight install/uninstall runs so the UI can cancel them.
    pub runs: Arc<crate::runner::RunRegistry>,
}

impl AppState {
    /// Build the state, deriving the settings path from `directories`.
    pub fn new(history: Option<History>, settings_path: PathBuf) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("toolpulse/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self {
            http,
            latest_cache: Arc::new(LatestCache::new()),
            history: Mutex::new(history),
            settings_path,
            scheduler: tokio::sync::Mutex::new(
                crate::scheduler_state::SchedulerState::default(),
            ),
            runs: Arc::new(crate::runner::RunRegistry::new()),
        }
    }
}
