//! macOS menu-bar tray icon.
//!
//! A single `TrayIcon` lives in the system menu bar (top-right on macOS).
//! Its menu summarizes every tool's status and offers quick actions:
//! show/hide the main window, re-scan, and quit. The menu is rebuilt whenever
//! a scan completes so the counts and version lines stay current.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    AppHandle, Emitter, Manager, Runtime, WebviewWindow,
};

/// The stable tray icon id; used to look it up later when rebuilding menus.
pub const TRAY_ID: &str = "toolpulse-tray";

/// Build and install the tray icon at startup.
///
/// The icon shows a small lightning glyph; left-clicking toggles the main
/// window, right-clicking opens the full menu (on platforms where left-click
/// is reserved).
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &[])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("missing icon"))
        .tooltip("Toolpulse — dev tools version monitor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event)
        .build(app)?;
    Ok(())
}

/// Rebuild the tray menu from a fresh list of tool statuses.
///
/// Called after every scan. We rebuild from scratch because Tauri's menu API
/// doesn't support in-place item mutation; recreating is cheap.
pub fn refresh(app: &AppHandle, statuses: &[crate::tools::ToolStatus]) {
    if let Ok(menu) = build_menu(app, statuses) {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_menu(Some(menu));
            // Update tooltip with a one-line summary.
            let outdated = statuses.iter().filter(|s| s.is_outdated).count();
            let tip = if outdated > 0 {
                format!("Toolpulse — {outdated} update(s) available")
            } else {
                "Toolpulse — all up to date".to_string()
            };
            let _ = tray.set_tooltip(Some(tip));
        }
    }
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" => {
            if let Some(win) = main_window(app) {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "hide" => {
            if let Some(win) = main_window(app) {
                let _ = win.hide();
            }
        }
        "rescan" => {
            // Trigger a scan via an event the frontend listens for.
            let _ = app.emit("toolpulse://rescan", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

fn on_tray_icon_event<R: Runtime>(
    tray: &tauri::tray::TrayIcon<R>,
    event: tauri::tray::TrayIconEvent,
) {
    // Left-click toggles the main window (matches the macOS convention where
    // the menu is opened by clicking and holding or right-clicking).
    if let tauri::tray::TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let app = tray.app_handle();
        if let Some(win) = main_window(app) {
            if win.is_visible().unwrap_or(false) {
                let _ = win.hide();
            } else {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
    }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window("main")
}

/// Construct the menu. Tool statuses are rendered as disabled informational
/// rows (a native menu can't show arbitrary widgets), grouped under a
/// "Tools" submenu so the top level stays uncluttered.
fn build_menu(app: &AppHandle, statuses: &[crate::tools::ToolStatus]) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri::menu::IsMenuItem;

    // --- Header line: overall summary ---
    let total = statuses.len();
    let outdated = statuses.iter().filter(|s| s.is_outdated).count();
    let missing = statuses.iter().filter(|s| s.installed_version.is_none()).count();
    let installed = total - missing;
    let up_to_date = installed - outdated;

    let header_text = if statuses.is_empty() {
        "Toolpulse — not scanned yet".to_string()
    } else {
        format!("{up_to_date} up to date · {outdated} updates · {missing} missing")
    };
    let header = MenuItem::with_id(app, "header", &header_text, false, None::<&str>)?;

    // Build all owned items in one scope so their borrows outlive `with_items`.
    let sep_after_header = PredefinedMenuItem::separator(app)?;

    // Per-tool rows, grouped by status (most actionable first).
    let mut sorted = statuses.to_vec();
    sorted.sort_by_key(|s| match status_kind(s) {
        Kind::Outdated => 0,
        Kind::Missing => 1,
        Kind::Unknown => 2,
        Kind::Updated => 3,
    });
    let tool_items: Vec<MenuItem<tauri::Wry>> = sorted
        .iter()
        .map(|s| {
            let line = format_tool_line(s);
            // Informational rows: disabled so clicking does nothing.
            MenuItem::with_id(app, &format!("tool_{}", s.name), &line, false, None::<&str>)
        })
        .collect::<Result<_, _>>()?;

    // The submenu takes ownership of a copy of the refs; the tool_items Vec
    // stays alive in this scope until the menu is built.
    let tool_refs: Vec<&dyn IsMenuItem<tauri::Wry>> = tool_items
        .iter()
        .map(|i| i as &dyn IsMenuItem<tauri::Wry>)
        .collect();
    let tools_submenu = if tool_refs.is_empty() {
        None
    } else {
        Some(Submenu::with_items(app, "Tools", true, &tool_refs)?)
    };

    // Action items.
    let sep_actions = PredefinedMenuItem::separator(app)?;
    let show = MenuItem::with_id(app, "show", "Show Toolpulse", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Toolpulse", true, None::<&str>)?;
    let rescan = MenuItem::with_id(app, "rescan", "Re-scan now", true, None::<&str>)?;
    let sep_quit = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Toolpulse", true, None::<&str>)?;

    // Assemble final list of references. Every referenced item must live in
    // THIS scope, which it does.
    let mut refs: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&header, &sep_after_header];
    if let Some(ref sub) = tools_submenu {
        refs.push(sub);
    }
    refs.extend([
        &sep_actions as &dyn IsMenuItem<tauri::Wry>,
        &show,
        &hide,
        &rescan,
        &sep_quit,
        &quit,
    ]);

    Menu::with_items(app, &refs)
}

enum Kind {
    Updated,
    Outdated,
    Missing,
    Unknown,
}

fn status_kind(s: &crate::tools::ToolStatus) -> Kind {
    if s.error.is_some() && s.installed_version.is_none() {
        Kind::Missing
    } else if s.installed_version.is_some() && s.latest_version.is_some() && s.is_outdated {
        Kind::Outdated
    } else if s.installed_version.is_some() && s.latest_version.is_none() {
        Kind::Unknown
    } else {
        Kind::Updated
    }
}

/// Render one tool as a menu line, e.g. `🦀 Rust   1.88.0 → 1.97.1`.
fn format_tool_line(s: &crate::tools::ToolStatus) -> String {
    let icon = s.icon.clone();
    let name = s.display_name.clone();
    match (&s.installed_version, &s.latest_version) {
        (Some(installed), Some(latest)) if s.is_outdated => {
            format!("{icon} {name}   {installed} → {latest}")
        }
        (Some(installed), _) => format!("{icon} {name}   {installed}"),
        _ => format!("{icon} {name}   not installed"),
    }
}
