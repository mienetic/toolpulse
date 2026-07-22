//! Background notification scheduler.
//!
//! A single long-running tokio task wakes on a poll interval and decides
//! whether to fire an OS notification based on the user's schedule. State
//! (last-fire time, today's count, dedupe set) lives in memory and resets
//! at local midnight.
//!
//! The scheduler is intentionally simple: it polls rather than computing exact
//! next-fire instants, because settings can change at any time and a poll loop
//! always re-reads the latest config from disk.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use tauri::AppHandle;
use tokio::time;

use crate::commands;
use crate::state::AppState;
use crate::tools::{NotifyMode, ToolStatus};

/// Spawn the scheduler. Returns immediately; the task runs for the app's life.
///
/// Uses `tauri::async_runtime::spawn` (rather than `tokio::spawn` directly) so
/// the task attaches to whatever runtime Tauri is running, regardless of which
/// thread `setup` is called from.
pub fn spawn(app: AppHandle, state: Arc<AppStateShared>) {
    tauri::async_runtime::spawn(async move {
        // Start at a short tick so the first fire happens promptly, then adapt
        // to the configured poll interval each iteration.
        let mut tick = Duration::from_secs(60);
        loop {
            time::sleep(tick).await;
            tick = run_once(&app, &state).await;
        }
    });
}

/// Pieces of `AppState` the scheduler needs, extracted so we don't pass the
/// whole Tauri `State` wrapper (which is request-scoped) into a long task.
pub struct AppStateShared {
    pub settings_path: PathBuf,
    pub inner: Arc<AppState>,
}

async fn run_once(app: &AppHandle, state: &Arc<AppStateShared>) -> Duration {
    let settings = crate::tools::settings::load(&state.settings_path);
    let notif = &settings.notifications;

    // Determine the poll interval for the next loop regardless of outcome.
    let next = Duration::from_secs(notif.poll_interval_secs());

    // Disabled entirely — nothing to do.
    if matches!(notif.mode, NotifyMode::Off) {
        return next;
    }

    // Quiet hours: skip silently.
    let now_hour = Local::now().format("%H").to_string().parse::<u32>().unwrap_or(0);
    if notif.is_quiet_hour(now_hour) {
        return next;
    }

    // Roll over the day counters at local midnight.
    {
        let mut s = state.inner.scheduler.lock().await;
        let today = Local::now().date_naive();
        if s.day.as_ref() != Some(&today) {
            s.day = Some(today);
            s.count_today = 0;
            s.notified_tools.clear();
        }

        // Enforce the daily cap before doing any expensive work.
        if s.count_today >= notif.max_per_day {
            return next;
        }

        // Enforce the interval cadence (modes that care about it).
        if matches!(notif.mode, NotifyMode::Interval | NotifyMode::Both) {
            if let Some(last) = s.last_fired {
                let elapsed = last.elapsed().as_secs();
                let needed = notif.interval_hours as u64 * 3600;
                if elapsed < needed {
                    return next;
                }
            }
        }
    }

    // Run a scan to get fresh data. This reuses the same logic the manual
    // "Refresh" button uses, including latest-version lookups and history.
    let statuses = commands::run_scan(&state.inner).await;

    // Update the tray menu too, so the menu bar reflects the scheduled scan.
    crate::tray::refresh(app, &statuses);

    // Decide what (if anything) to notify about.
    let to_notify: Vec<&ToolStatus> = statuses
        .iter()
        .filter(|s| {
            if notif.only_updates && !s.is_outdated {
                return false;
            }
            true
        })
        .collect();

    if to_notify.is_empty() {
        // No candidates; don't consume the daily budget, but record that we
        // checked so the interval still advances.
        let mut s = state.inner.scheduler.lock().await;
        // Only advance the timer in interval mode; daily-count mode should
        // still try again on the next poll within its budget.
        if matches!(notif.mode, NotifyMode::Interval | NotifyMode::Both) {
            s.last_fired = Some(Instant::now());
        }
        return next;
    }

    // Apply dedupe: drop tools already notified today, if enabled.
    let mut s = state.inner.scheduler.lock().await;
    let candidates: Vec<&ToolStatus> = if notif.dedupe_same_day {
        to_notify
            .iter()
            .copied()
            .filter(|t| !s.notified_tools.contains(&t.name))
            .collect()
    } else {
        to_notify.clone()
    };

    if candidates.is_empty() {
        // Everything was already reported today.
        if matches!(notif.mode, NotifyMode::Interval | NotifyMode::Both) {
            s.last_fired = Some(Instant::now());
        }
        return next;
    }

    // Fire a single grouped notification for all candidates.
    fire_notification(app, &candidates);

    // Bookkeeping.
    s.count_today += 1;
    s.last_fired = Some(Instant::now());
    for c in &candidates {
        s.notified_tools.insert(c.name.clone());
    }

    next
}

fn fire_notification(app: &AppHandle, candidates: &[&ToolStatus]) {
    use tauri_plugin_notification::NotificationExt;
    let title = format!("Toolpulse: {} update(s) available", candidates.len());
    let body = candidates
        .iter()
        .map(|s| {
            format!(
                "{} {} → {}",
                s.display_name,
                s.installed_version.as_deref().unwrap_or("?"),
                s.latest_version.as_deref().unwrap_or("?"),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Err(e) = app.notification().builder().title(&title).body(&body).show() {
        log::warn!("scheduled notification failed: {e}");
    }
}
