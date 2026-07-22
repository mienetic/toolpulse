//! Runs `tool --version` on the local machine and parses the output.
//!
//! All checks run concurrently via `futures::future::join_all`, so scanning
//! the full registry completes in roughly the time of the slowest single tool.

use std::process::Stdio;

use chrono::Utc;
use futures::future::join_all;
use tokio::process::Command;

use super::types::*;

/// Run every definition's version check concurrently.
///
/// `selected_paths` lets the caller pin a specific binary path per tool when
/// multiple installations exist. Keys are tool names, values are absolute
/// binary paths chosen by the user.
pub async fn check_all(
    definitions: &[ToolDefinition],
    selected_paths: &std::collections::HashMap<String, String>,
) -> Vec<ToolStatus> {
    let futures: Vec<_> = definitions
        .iter()
        .map(|def| {
            let selected = selected_paths.get(&def.name).map(String::as_str);
            check_one(def, selected)
        })
        .collect();
    join_all(futures).await
}

/// Inspect a single tool: resolve the binary on PATH, run it, parse output.
///
/// When `def.detect_versions` is set, every installation on the machine is
/// enumerated (Homebrew / nvm / pyenv / system) so the user can pick which to
/// track. If `selected_path` is provided and matches a known installation,
/// that copy is reported as `installed_version`/`path`; otherwise the active
/// PATH entry is used.
pub async fn check_one(def: &ToolDefinition, selected_path: Option<&str>) -> ToolStatus {
    let now = Utc::now().timestamp();

    // Enumerate every installation when multi-version detection is enabled;
    // otherwise just resolve the first PATH hit.
    let mut installs = if def.detect_versions {
        super::versions::detect_all(def).await
    } else {
        // Single-version fast path: resolve once and treat as the only install.
        match resolve_first(&def.binary).await {
            Some(path) => {
                let version = run_version(&path, &def.args, &def.parser).await;
                vec![ToolInstallation {
                    path,
                    version,
                    source: InstallationSource::Unknown,
                    is_active: true,
                }]
            }
            None => Vec::new(),
        }
    };

    if installs.is_empty() {
        return ToolStatus::missing(def, format!("{} not found on PATH", def.binary));
    }

    // Decide which installation to surface as the tracked one. If the user
    // pinned a path and it exists in our list, mark it active and the rest
    // inactive. Otherwise the first (highest PATH priority) stays active.
    if let Some(chosen) = selected_path {
        let mut found = false;
        for inst in installs.iter_mut() {
            let is_chosen = inst.path == chosen;
            inst.is_active = is_chosen;
            if is_chosen {
                found = true;
            }
        }
        // If the chosen path isn't in the detected list (e.g. installed after
        // the scan started), probe it directly and prepend it as active.
        if !found {
            if let Some(version) = run_version(chosen, &def.args, &def.parser).await {
                for inst in installs.iter_mut() {
                    inst.is_active = false;
                }
                installs.insert(
                    0,
                    ToolInstallation {
                        path: chosen.to_string(),
                        version: Some(version),
                        source: super::versions::infer_source_public(chosen),
                        is_active: true,
                    },
                );
            }
        }
    }

    // Find the active installation (only one should be active at this point).
    let active = installs
        .iter()
        .find(|i| i.is_active)
        .or_else(|| installs.first())
        .expect("at least one installation exists");
    let active_path = active.path.clone();
    let active_version = active.version.clone();

    match &active_version {
        Some(version) => ToolStatus {
            name: def.name.clone(),
            display_name: def.display_name.clone(),
            category: def.category,
            icon: def.icon.clone(),
            color: def.color.clone(),
            installed_version: Some(version.clone()),
            latest_version: None, // filled in by latest.rs
            is_outdated: false,
            path: Some(active_path),
            installations: installs,
            checked_at: now,
            error: None,
        },
        None => ToolStatus {
            name: def.name.clone(),
            display_name: def.display_name.clone(),
            category: def.category,
            icon: def.icon.clone(),
            color: def.color.clone(),
            installed_version: None,
            latest_version: None,
            is_outdated: false,
            path: Some(active_path),
            installations: installs,
            checked_at: now,
            error: Some("could not parse version from active binary".into()),
        },
    }
}

/// Resolve the first PATH match for `binary` (equivalent to plain `which`).
async fn resolve_first(binary: &str) -> Option<String> {
    let out = Command::new("which")
        .arg(binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

/// Run `<path> <args>` and extract a version string.
async fn run_version(
    path: &str,
    args: &[String],
    parser: &VersionParser,
) -> Option<String> {
    let out = Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .ok()?;
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    parse_version(&combined, parser)
}

/// Extract the first version-looking token from arbitrary command output.
///
/// A "version-looking token" is `\d+(\.\d+){0,3}`. A leading prefix such as
/// `v`, `go`, or `python ` is stripped first so the numeric run is found.
pub fn parse_version(output: &str, parser: &VersionParser) -> Option<String> {
    let normalized = match parser {
        VersionParser::StripPrefix { prefix } => output.trim_start_matches(prefix.as_str()).to_string(),
        VersionParser::FirstNumeric => output.to_string(),
    };

    let bytes = normalized.as_bytes();
    let mut start = None;
    let mut end = 0usize;
    let mut dots = 0u8;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
            end = i + 1;
        } else if b == b'.' && start.is_some() && dots < 3 {
            end = i + 1;
            dots += 1;
        } else if start.is_some() {
            break;
        }
    }
    let start = start?;
    if end <= start {
        return None;
    }
    let v = &normalized[start..end];
    Some(v.trim_end_matches('.').to_string())
}

/// Compare two version strings of the form `MAJOR.MINOR[.PATCH[.BUILD]]`.
///
/// Returns `true` when `installed` is strictly older than `latest`.
/// Non-numeric components fall back to lexicographic comparison.
pub fn is_outdated(installed: &str, latest: &str) -> bool {
    let a = strip_version_prefix(installed);
    let b = strip_version_prefix(latest);
    let ai: Vec<&str> = a.split('.').collect();
    let bi: Vec<&str> = b.split('.').collect();
    let n = ai.len().max(bi.len());
    for i in 0..n {
        let x = ai.get(i).copied().unwrap_or("0");
        let y = bi.get(i).copied().unwrap_or("0");
        match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(xi), Ok(yi)) => {
                if xi != yi {
                    return xi < yi;
                }
            }
            _ => {
                if x != y {
                    return x < y;
                }
            }
        }
    }
    false
}

/// Drop any leading non-numeric characters and any pre-release suffix.
///
/// e.g. `"v1.2.3-rc1"` -> `"1.2.3"`, `"python 3.9.12"` -> `"3.9.12"`.
fn strip_version_prefix(s: &str) -> &str {
    let trimmed = s.trim_start_matches(|c: char| !c.is_ascii_digit());
    trimmed.split('-').next().unwrap_or(trimmed)
}
