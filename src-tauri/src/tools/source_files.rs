//! Scans for standalone source files (scripts, snippets, throwaway code) that
//! live outside any recognized project.
//!
//! We reuse the project scanner's skip list (node_modules, .git, target, …) and
//! also skip directories that contain a project manifest, so this view shows
//! only "loose" files the user might otherwise lose track of.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tauri::Emitter;
use tokio::fs;

use super::types::*;

/// Directory basenames to never descend into. Only dependency caches, VCS,
/// and build artifacts — we intentionally do NOT skip user content dirs like
/// Documents, Desktop, or Downloads because users keep scripts there.
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", ".svn", ".hg", "target", "dist",
    ".next", ".nuxt", ".cache", ".gradle", ".m2", "venv", ".venv", "env",
    "__pycache__", ".tox", ".pytest_cache", "vendor", ".cargo", ".rustup",
    "Pods", "DerivedData", ".deno", ".bun", "coverage", ".turbo",
    // System/media dirs that genuinely won't hold source.
    "Library", "Pictures", "Music", "Movies", ".Trash",
];

/// Manifest files whose presence means "this is a project, skip its files".
const PROJECT_MARKERS: &[&str] = &[
    "package.json", "Cargo.toml", "go.mod", "pyproject.toml", "Pipfile",
    "requirements.txt", "Gemfile", "composer.json", "pom.xml", "build.gradle",
    "build.gradle.kts",
];

/// Scan `root` for standalone source files matching `languages`.
///
/// Emits `toolpulse://source-file` events as each file is found (progressive)
/// and returns the full list at the end.
pub async fn scan_source_files(
    root: &Path,
    languages: &[SourceLanguage],
    app: &tauri::AppHandle,
) -> Vec<SourceFile> {
    let exts: HashSet<&str> = languages
        .iter()
        .flat_map(|l| l.extensions().iter().copied())
        .collect();
    let lang_for_ext: Vec<(&str, SourceLanguage)> = languages
        .iter()
        .flat_map(|l| l.extensions().iter().map(move |e| (*e, *l)))
        .collect();

    let mut found: Vec<SourceFile> = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    // FIFO queue for breadth-first traversal.
    use std::collections::VecDeque;
    let mut queue: VecDeque<PathBuf> = [root.to_path_buf()].into_iter().collect();

    while let Some(dir) = queue.pop_front() {
        if visited.contains(&dir) {
            continue;
        }
        visited.insert(dir.clone());

        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        // First pass: detect if this dir is a project root.
        let mut is_project = false;
        let mut child_dirs: Vec<PathBuf> = Vec::new();
        let mut files_here: Vec<(String, PathBuf)> = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let ft = match entry.file_type().await {
                Ok(t) => t,
                Err(_) => continue,
            };
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if ft.is_dir() {
                if !SKIP_DIRS.contains(&name_str.as_ref()) && !name_str.starts_with('.') {
                    child_dirs.push(entry.path());
                }
            } else if ft.is_file() {
                if PROJECT_MARKERS.contains(&name_str.as_ref()) {
                    is_project = true;
                }
                files_here.push((name_str.to_string(), entry.path()));
            }
        }

        // If this directory is a project root, don't surface its files here —
        // they belong to the Projects view.
        if !is_project {
            for (name, path) in &files_here {
                if let Some(ext) = name.rsplit('.').next() {
                    if exts.contains(ext) {
                        let lang = lang_for_ext
                            .iter()
                            .find(|(e, _)| *e == ext)
                            .map(|(_, l)| *l);
                        if let Some(language) = lang {
                            let size = file_size(&path).await.unwrap_or(0);
                            let file = SourceFile {
                                path: path.to_string_lossy().to_string(),
                                name: name.clone(),
                                language,
                                size_bytes: size,
                            };
                            let _ = app.emit("toolpulse://source-file", file.clone());
                            found.push(file);
                        }
                    }
                }
            }
        }

        for child in child_dirs.into_iter().rev() {
            queue.push_back(child);
        }

        if found.len() >= 5000 {
            break;
        }
    }

    let _ = app.emit("toolpulse://source-files-done", found.len());
    found
}

async fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).await.ok().map(|m| m.len())
}
