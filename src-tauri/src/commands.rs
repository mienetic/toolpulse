//! Tauri command handlers — the IPC surface exposed to the React frontend.
//!
//! These functions are thin: they pull shared state (HTTP client, history DB,
//! settings path, latest cache) from Tauri's managed state and delegate to
//! the `tools` module. Heavy work runs on the tokio runtime Tauri provides.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::state::AppState;
use crate::tools::{
    self, active_definitions, check_one, fetch_latest, find, is_outdated, list_packages,
    ActionKind, DashboardSummary, InstalledPackage, Settings, Snapshot, ToolStatus,
};

/// Scan every enabled tool and enrich each result with its latest version.
///
/// Also persists a snapshot to history so the UI can show trends.
pub async fn run_scan(state: &AppState) -> Vec<ToolStatus> {
    let settings = tools::settings::load(&state.settings_path);
    let defs = active_definitions(&settings);

    // 1. Local version checks run concurrently, honoring any user-pinned
    //    installation paths.
    let mut statuses = tools::check_all(&defs, &settings.selected_paths).await;

    // 2. Enrich with latest versions (also concurrent). We look the definition
    //    up by name inside each future so the future owns the data it needs
    //    rather than borrowing from `defs`.
    let cache = state.latest_cache.clone();
    let http = state.http.clone();
    let latest_futs: Vec<_> = statuses
        .iter()
        .map(|s| {
            let name = s.name.clone();
            let def = defs.iter().find(|d| d.name == name).cloned();
            let cache = cache.clone();
            let http = http.clone();
            async move {
                let latest = match def {
                    Some(def) if s.installed_version.is_some() => {
                        fetch_latest(&def, &cache, &http).await
                    }
                    _ => None,
                };
                (name, latest)
            }
        })
        .collect();
    let latest_results = futures::future::join_all(latest_futs).await;
    for (name, latest) in latest_results {
        if let Some(s) = statuses.iter_mut().find(|s| s.name == name) {
            s.latest_version = latest.clone();
            if let (Some(installed), Some(latest)) = (&s.installed_version, &latest) {
                s.is_outdated = is_outdated(installed, latest);
            }
        }
    }

    // 3. Persist to history (best-effort; ignore DB errors).
    let history_guard = state.history.lock().await;
    if let Some(history) = history_guard.as_ref() {
        let _ = history.record(&statuses).await;
    }

    statuses
}

#[tauri::command]
pub async fn check_all_tools(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ToolStatus>, String> {
    let statuses = run_scan(&state).await;
    // Keep the menu-bar tray in sync with the latest results.
    crate::tray::refresh(&app, &statuses);
    Ok(statuses)
}

#[tauri::command]
pub async fn check_tool(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ToolStatus, String> {
    let def = find(&name).ok_or_else(|| format!("unknown tool: {name}"))?;
    // Honor a user-pinned installation path if one is set for this tool.
    let settings = tools::settings::load(&state.settings_path);
    let selected = settings.selected_paths.get(&name).map(String::as_str);
    let mut status = check_one(&def, selected).await;
    if status.installed_version.is_some() {
        if let Some(latest) = fetch_latest(&def, &state.latest_cache, &state.http).await {
            if let Some(installed) = &status.installed_version {
                status.is_outdated = is_outdated(installed, &latest);
            }
            status.latest_version = Some(latest);
        }
    }
    Ok(status)
}

#[tauri::command]
pub async fn get_latest_version(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let def = find(&name).ok_or_else(|| format!("unknown tool: {name}"))?;
    Ok(fetch_latest(&def, &state.latest_cache, &state.http).await)
}

#[tauri::command]
pub async fn list_installed_packages(
    name: String,
) -> Result<Vec<InstalledPackage>, String> {
    let def = find(&name).ok_or_else(|| format!("unknown tool: {name}"))?;
    Ok(list_packages(&def).await)
}

#[tauri::command]
pub async fn get_history(
    days: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Snapshot>, String> {
    let history = state.history.lock().await;
    match history.as_ref() {
        Some(h) => h.recent(days).await.map_err(|e| e.to_string()),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(tools::settings::load(&state.settings_path))
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    tools::settings::save(&state.settings_path, &settings)
}

#[tauri::command]
pub fn dashboard_summary(statuses: Vec<ToolStatus>) -> DashboardSummary {
    let total = statuses.len();
    let installed = statuses.iter().filter(|s| s.installed_version.is_some()).count();
    let outdated = statuses.iter().filter(|s| s.is_outdated).count();
    let missing = statuses.iter().filter(|s| s.installed_version.is_none()).count();
    DashboardSummary {
        total,
        installed,
        outdated,
        missing,
    }
}

/// Fire OS notifications for every outdated tool.
///
/// Uses the notification plugin registered on the app. Failures (e.g. denied
/// permission) are logged but never propagated, since this is best-effort UX.
#[tauri::command]
pub async fn notify_outdated(
    app: AppHandle,
    statuses: Vec<ToolStatus>,
) -> Result<usize, String> {
    use tauri_plugin_notification::NotificationExt;
    let outdated: Vec<&ToolStatus> = statuses.iter().filter(|s| s.is_outdated).collect();
    if outdated.is_empty() {
        return Ok(0);
    }
    let title = format!("Toolpulse: {} update(s) available", outdated.len());
    let body = outdated
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
        log::warn!("notification failed: {e}");
    }
    Ok(outdated.len())
}

/// Resolve the absolute path used to store the settings JSON.
///
/// Exposed so the frontend can display "where is my config?" in the UI.
#[tauri::command]
pub fn settings_path(state: State<'_, Arc<AppState>>) -> Result<PathBuf, String> {
    Ok(state.settings_path.clone())
}

// --- Tool / package management (install / uninstall / upgrade) -------------

/// Run an install/uninstall/upgrade command for a builtin tool, streaming
/// output to the frontend. Returns Ok on success, Err with a message on
/// failure (including cancellation).
#[tauri::command]
pub async fn manage_tool(
    app: AppHandle,
    name: String,
    action: crate::tools::ActionKind,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::runner::{action_label, run_streaming};
    use crate::tools::ActionKind;

    let def = find(&name).ok_or_else(|| format!("unknown tool: {name}"))?;
    let actions = def
        .actions
        .ok_or_else(|| format!("{} has no install/uninstall actions", def.display_name))?;
    let argv: Vec<String> = match action {
        ActionKind::Install => actions.install,
        ActionKind::Uninstall => actions.uninstall,
        ActionKind::Upgrade => actions.upgrade,
    };
    if argv.is_empty() {
        return Err(format!("{} cannot be {}", def.display_name, label_verb(action)));
    }

    let tag = format!("tool:{name}");
    let cancel = state.runs.take_cancel_flag();
    let result = run_streaming(&app, &tag, &argv, cancel).await;

    // Fire an OS notification with the outcome.
    let subject = &def.display_name;
    let err = result.as_ref().err();
    notify_result(&app, &action_label(action, subject), err);

    result
}

/// Install or uninstall a global package belonging to a tool's package
/// manager (e.g. `npm install -g typescript`, `cargo install ripgrep`).
#[tauri::command]
pub async fn manage_package(
    app: AppHandle,
    tool: String,
    package: String,
    action: crate::tools::ActionKind,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::runner::{action_label, run_streaming};
    use crate::tools::ActionKind;

    let argv = package_argv(&tool, &package, action)
        .ok_or_else(|| format!("{tool} cannot manage package {package}"))?;

    let tag = format!("pkg:{tool}:{package}");
    let cancel = state.runs.take_cancel_flag();
    let result = run_streaming(&app, &tag, &argv, cancel).await;

    let subject = &format!("{package} ({tool})");
    let err = result.as_ref().err();
    notify_result(&app, &action_label(action, subject), err);

    result
}

/// Cancel the currently-running install/uninstall/upgrade command.
#[tauri::command]
pub fn cancel_run(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.runs.cancel();
    Ok(())
}

/// Build the argv for a package-manager operation.
///
/// Returns `None` for tools that have no package manager or for unsupported
/// actions (e.g. upgrading a pip package isn't a single safe command).
fn package_argv(tool: &str, package: &str, action: ActionKind) -> Option<Vec<String>> {
    use crate::tools::PackageSource;
    let def = find(tool)?;
    let src = def.packages?;
    let toks: Vec<String> = match (src, action) {
        (PackageSource::NpmGlobal, ActionKind::Install) => strs(&["npm", "install", "-g", package]),
        (PackageSource::NpmGlobal, ActionKind::Uninstall) => strs(&["npm", "uninstall", "-g", package]),
        (PackageSource::NpmGlobal, ActionKind::Upgrade) => {
            strs(&["npm", "install", "-g", &format!("{package}@latest")])
        }
        (PackageSource::CargoInstall, ActionKind::Install) => strs(&["cargo", "install", package]),
        (PackageSource::CargoInstall, ActionKind::Uninstall) => strs(&["cargo", "uninstall", package]),
        (PackageSource::CargoInstall, ActionKind::Upgrade) => {
            strs(&["cargo", "install", package, "--force"])
        }
        (PackageSource::PipList, ActionKind::Install) => strs(&["pip3", "install", package]),
        (PackageSource::PipList, ActionKind::Uninstall) => strs(&["pip3", "uninstall", "-y", package]),
        (PackageSource::PipList, ActionKind::Upgrade) => {
            strs(&["pip3", "install", "--upgrade", package])
        }
        (PackageSource::GemList, ActionKind::Install) => strs(&["gem", "install", package]),
        (PackageSource::GemList, ActionKind::Uninstall) => strs(&["gem", "uninstall", package]),
        (PackageSource::GemList, ActionKind::Upgrade) => strs(&["gem", "update", package]),
        (PackageSource::ComposerGlobal, ActionKind::Install) => {
            strs(&["composer", "global", "require", package])
        }
        (PackageSource::ComposerGlobal, ActionKind::Uninstall) => {
            strs(&["composer", "global", "remove", package])
        }
        (PackageSource::BrewList, ActionKind::Install) => strs(&["brew", "install", package]),
        (PackageSource::BrewList, ActionKind::Uninstall) => strs(&["brew", "uninstall", package]),
        (PackageSource::BrewList, ActionKind::Upgrade) => strs(&["brew", "upgrade", package]),
        _ => return None,
    };
    Some(toks)
}

fn strs(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

fn label_verb(action: crate::tools::ActionKind) -> &'static str {
    match action {
        crate::tools::ActionKind::Install => "installed this way",
        crate::tools::ActionKind::Uninstall => "uninstalled this way",
        crate::tools::ActionKind::Upgrade => "upgraded this way",
    }
}

/// Fire a success/failure OS notification for a completed run.
fn notify_result(app: &AppHandle, label: &str, err: Option<&String>) {
    use tauri_plugin_notification::NotificationExt;
    let (title, body) = match err {
        None => ("Toolpulse".to_string(), format!("✓ {label}")),
        Some(msg) => ("Toolpulse error".to_string(), format!("✗ {label}: {msg}")),
    };
    if let Err(e) = app.notification().builder().title(&title).body(&body).show() {
        log::warn!("result notification failed: {e}");
    }
}

// --- Project scanning -------------------------------------------------------

/// Scan a directory for projects. When `root` is `None`, defaults to `$HOME`.
///
/// Walks the tree looking for manifests (package.json, Cargo.toml, etc.),
/// skipping dependency caches and VCS dirs for speed. Returns a flat list of
/// discovered projects with summary metadata.
#[tauri::command]
pub async fn scan_projects(
    root: Option<String>,
) -> Result<Vec<crate::tools::DiscoveredProject>, String> {
    let root = match root {
        Some(r) => std::path::PathBuf::from(r),
        None => std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .map_err(|_| "$HOME is not set".to_string())?,
    };
    Ok(crate::tools::projects::scan(&root).await)
}

/// Resolve the default set of roots for a "scan the whole machine" run.
///
/// Covers the directories where users actually keep projects, while avoiding
/// the OS/system trees that would make a scan of `/` pathologically slow and
/// full of false positives.
pub fn machine_roots() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home));
    }
    // Common project locations outside $HOME.
    for candidate in ["/Applications", "/Volumes", "/opt/homebrew", "/usr/local"] {
        let p = PathBuf::from(candidate);
        if p.is_dir() {
            roots.push(p);
        }
    }
    roots
}

/// Scan the whole machine (progressive). Emits `toolpulse://project` events
/// as each project is discovered and a final `toolpulse://scan-done` event,
/// so the UI can populate incrementally instead of blocking on a long scan.
///
/// Returns the full list at the end as well, for callers that want the
/// complete snapshot.
#[tauri::command]
pub async fn scan_machine(
    app: AppHandle,
) -> Result<Vec<crate::tools::DiscoveredProject>, String> {
    use tauri::Emitter;
    let roots = machine_roots();
    let app_for_projects = app.clone();

    let on_project = move |proj: &crate::tools::DiscoveredProject| {
        let _ = app_for_projects.emit("toolpulse://project", proj.clone());
    };
    let on_dir = |_dir: &str| {
        // Could emit a "scanning X" event; intentionally omitted to avoid
        // flooding the IPC channel on large trees.
    };

    let result = crate::tools::projects::scan_roots(&roots, &on_dir, &on_project).await;
    let _ = app.emit("toolpulse://scan-done", result.len());
    Ok(result)
}

/// Open a native folder-picker dialog and return the chosen path.
///
/// Uses `tauri-plugin-dialog` so the picker is native on every platform
/// (macOS NSOpenPanel, Windows IFileOpenDialog, Linux GTK dialog).
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select a folder to scan for projects")
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let result = rx.await.map_err(|e| e.to_string())?;
    Ok(result.and_then(|p| p.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

/// Parse a project's manifest into a detailed dependency list, then check
/// each Node dependency against the npm registry to flag outdated ones.
#[tauri::command]
pub async fn scan_project_deps(
    manifest: String,
    ecosystem: crate::tools::ProjectEcosystem,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::tools::ProjectDependency>, String> {
    let path = std::path::PathBuf::from(&manifest);
    let mut deps = crate::tools::projects::parse_deps_detail(&path, ecosystem).await;
    crate::tools::projects::check_outdated(&mut deps, &state.http, ecosystem).await;
    Ok(deps)
}

// --- Folder / IDE actions ---------------------------------------------------

/// A detected editor/IDE the user can open projects in.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectedIde {
    /// Stable id, e.g. `"code"`, `"cursor"`, `"zed"`.
    pub id: String,
    /// Display name, e.g. `"VS Code"`.
    pub name: String,
    /// Executable or `open -a` app name used to launch it.
    pub command: String,
    /// `true` when the launch is `open -a <app>` (macOS .app), `false` for a
    /// bare CLI command.
    pub is_app: bool,
}

/// Detect installed editors/IDEs by probing for their CLI command or .app.
///
/// Returns every editor we can confidently launch, in a stable order. The
/// frontend offers these as a dropdown on each project row.
#[tauri::command]
pub async fn detect_ides() -> Result<Vec<DetectedIde>, String> {
    let mut ides = Vec::new();

    // Each entry: (id, display name, cli command, macOS .app name).
    // We detect via the CLI command when present (cross-platform), else fall
    // back to checking /Applications for the .app on macOS.
    let candidates: &[(&str, &str, &str, &str)] = &[
        ("code", "VS Code", "code", "Visual Studio Code.app"),
        ("code-insiders", "VS Code Insiders", "code-insiders", "Visual Studio Code - Insiders.app"),
        ("cursor", "Cursor", "cursor", "Cursor.app"),
        ("zed", "Zed", "zeditor", "Zed.app"),
        ("subl", "Sublime Text", "subl", "Sublime Text.app"),
        ("nova", "Nova", "nova", "Nova.app"),
        ("vim", "Vim (terminal)", "vim", ""),
        ("emacs", "Emacs", "emacs", ""),
    ];

    for (id, name, cli, app) in candidates {
        let found = if command_exists(cli).await {
            true
        } else if !app.is_empty() {
            #[cfg(target_os = "macos")]
            {
                std::path::Path::new("/Applications").join(app).is_dir()
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        } else {
            false
        };
        if found {
            // Launch preference: CLI command if available, else `open -a <app>`.
            let (command, is_app) = if command_exists(cli).await {
                (cli.to_string(), false)
            } else {
                (app.to_string(), true)
            };
            ides.push(DetectedIde {
                id: id.to_string(),
                name: name.to_string(),
                command,
                is_app,
            });
        }
    }

    Ok(ides)
}

/// Check whether a command is resolvable on PATH.
async fn command_exists(cmd: &str) -> bool {
    tokio::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Reveal a folder in the native file manager (Finder on macOS).
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    open_in_file_manager(&path)
        .map_err(|e| e.to_string())
}

/// Open a project folder in the chosen editor/IDE.
///
/// `ide_command` is the command to run; when `is_app` is true we launch via
/// `open -a <app> <path>` (macOS .app), otherwise `<command> <path>`.
#[tauri::command]
pub async fn open_in_ide(
    path: String,
    ide_command: String,
    is_app: bool,
) -> Result<(), String> {
    let result = if is_app {
        tokio::process::Command::new("open")
            .args(["-a", &ide_command, &path])
            .status()
            .await
    } else {
        tokio::process::Command::new(&ide_command)
            .arg(&path)
            .status()
            .await
    };
    result.map_err(|e| e.to_string())?;
    Ok(())
}

/// Move a folder to the system trash (recoverable, not permanent delete).
///
/// On macOS we use the Finder-backed AppleScript so the item lands in Trash
/// rather than being unlinked directly; on other platforms we fall back to
/// the `trash` crate behavior via `mv` to the trash dir as a best effort.
#[tauri::command]
pub async fn trash_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }

    #[cfg(target_os = "macos")]
    {
        // osascript tells Finder to delete, which moves to Trash safely.
        let script = format!(
            "tell application \"Finder\" to delete (POSIX file \"{}\")",
            path.replace('"', "\\\"")
        );
        let out = tokio::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // PowerShell's Microsoft.VisualBasic.FileIO.FileSystem.DeleteDirectory
        // sends the folder to the Recycle Bin (recoverable).
        let ps = format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path.replace('\'', "''")
        );
        let out = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Linux: use the `trash-put` command from trash-cli if available.
        let _ = &p;
        let out = tokio::process::Command::new("trash-put")
            .arg(&path)
            .status()
            .await;
        match out {
            Ok(s) if s.success() => Ok(()),
            _ => Err("install 'trash-cli' to enable trash on Linux".into()),
        }
    }
}

/// Open a new terminal window at `path` (cd into it).
///
/// macOS → Terminal.app via AppleScript; Linux → a best-effort `x-terminal-
/// emulator`; Windows → `cmd` / PowerShell.
#[tauri::command]
pub async fn open_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\"\n\
             activate\n\
             do script \"cd \\\"{}\\\"\"\n\
             end tell",
            path.replace('"', "\\\"")
        );
        let out = tokio::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        return tokio::process::Command::new("x-terminal-emulator")
            .args(["-e", &format!("cd '{path}'")])
            .status()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    #[cfg(windows)]
    {
        return tokio::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &format!("cd /d {path}")])
            .status()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = path;
        Err("open_terminal is not supported on this platform".into())
    }
}

/// Open `path` in the native file manager. macOS → Finder, others → `xdg-open`.
fn open_in_file_manager(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).status()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).status()?;
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(path).status()?;
    }
    Ok(())
}

// --- Standalone source-file scanning ----------------------------------------

/// Scan the machine for standalone source files (scripts/snippets) outside any
/// recognized project. Emits `toolpulse://source-file` events progressively.
#[tauri::command]
pub async fn scan_source_files(
    app: AppHandle,
    languages: Vec<crate::tools::SourceLanguage>,
) -> Result<Vec<crate::tools::SourceFile>, String> {
    let root = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| "$HOME is not set".to_string())?;
    // Default to all languages when none are requested.
    let langs: Vec<crate::tools::SourceLanguage> = if languages.is_empty() {
        crate::tools::SourceLanguage::all().to_vec()
    } else {
        languages
    };
    Ok(crate::tools::source_files::scan_source_files(&root, &langs, &app).await)
}

// --- Auto-updater -----------------------------------------------------------

/// Check for a newer release on GitHub.
///
/// Returns `Some(version)` when an update is available, `None` when the app
/// is already up to date.
#[derive(serde::Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub body: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("updater init failed: {e}"))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            body: update.body.clone(),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            body: None,
        }),
        Err(e) => Err(format!("update check failed: {e}")),
    }
}

/// Download and install the latest update, then restart the app.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("updater init failed: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("update check failed: {e}"))?
        .ok_or("no update available")?;
    // Download and install. The installer handles platform-specific logic.
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("install failed: {e}"))?;
    // Restart the app to apply the update.
    app.restart();
}

/// Collect system info for bug reports (OS, arch, app version).
#[tauri::command]
pub fn system_info() -> serde_json::Value {
    serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "home": std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default(),
    })
}
