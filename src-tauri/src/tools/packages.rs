//! Enumerates globally installed packages for package-managing tools.
//!
//! Each `PackageSource` variant runs a different command and parses its
//! output format. All parsing is defensive: malformed output yields an empty
//! list rather than an error, so a broken tool never blocks the UI.

use std::process::Stdio;

use serde_json::Value;
use tokio::process::Command;

use super::types::*;

/// List installed packages for the given tool, if it has a package source,
/// then measure each package's on-disk size.
pub async fn list_packages(def: &ToolDefinition) -> Vec<InstalledPackage> {
    let Some(src) = &def.packages else {
        return Vec::new();
    };
    let mut packages = match src {
        PackageSource::NpmGlobal => run_npm_global().await,
        PackageSource::CargoInstall => run_cargo_install().await,
        PackageSource::PipList => run_pip_list().await,
        PackageSource::GoModules => run_go_modules().await,
        PackageSource::BunGlobal => run_bun_global().await,
        PackageSource::DenoModules => run_deno_modules().await,
        PackageSource::GemList => run_gem_list().await,
        PackageSource::ComposerGlobal => run_composer_global().await,
        PackageSource::BrewList => run_brew_list().await,
    };
    // Fill in sizes (mixed: manager query + du fallback). Best-effort: a
    // failure to measure one package never blocks the list.
    super::sizes::measure_sizes(&mut packages).await;
    packages
}

async fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

async fn run_npm_global() -> Vec<InstalledPackage> {
    // `npm ls -g --depth=0 --json` returns a nested object under "dependencies".
    let body = match run("npm", &["ls", "-g", "--depth=0", "--json"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let value: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(deps) = value.get("dependencies").and_then(|d| d.as_object()) {
        for (name, info) in deps {
            let version = info.get("version").and_then(|v| v.as_str()).map(String::from);
            out.push(InstalledPackage {
                name: name.trim_start_matches('@').to_string(),
                version,
                size_bytes: None,
                    manager: "npm".into(),
            });
        }
    }
    out
}

async fn run_cargo_install() -> Vec<InstalledPackage> {
    // Output looks like:
    //   ripgrep v13.0.0:
    //       ripgrep v13.0.0
    let body = match run("cargo", &["install", "--list"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        // Top-level entries have no leading whitespace and contain " v".
        if line.is_empty() || line.starts_with(' ') {
            continue;
        }
        if let Some(idx) = line.find(" v") {
            let name = line[..idx].to_string();
            let rest = &line[idx + 2..];
            let version = rest.split_whitespace().next().map(String::from);
            out.push(InstalledPackage {
                name,
                version,
                size_bytes: None,
                    manager: "cargo".into(),
            });
        }
    }
    out
}

async fn run_pip_list() -> Vec<InstalledPackage> {
    let body = match run("pip3", &["list", "--format=json"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let value: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name").and_then(|n| n.as_str())?.to_string();
                    let version = item.get("version").and_then(|v| v.as_str()).map(String::from);
                    Some(InstalledPackage {
                        name,
                        version,
                        size_bytes: None,
                    manager: "pip".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn run_go_modules() -> Vec<InstalledPackage> {
    // `go list -m all` only works inside a module dir; we surface the Go
    // version itself as the single "package" so the list isn't empty.
    let version = run("go", &["version"]).await.unwrap_or_default();
    let v = version
        .split_whitespace()
        .nth(2)
        .map(|s| s.trim_start_matches("go").to_string());
    vec![InstalledPackage {
        name: "go".into(),
        version: v,
        size_bytes: None,
                    manager: "go".into(),
    }]
}

async fn run_bun_global() -> Vec<InstalledPackage> {
    let body = match run("bun", &["pm", "ls", "-g"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    // Best-effort line parse: "<name>@<version>" or "<name>v<version>".
    let mut out = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(at) = trimmed.rfind('@') {
            let name = trimmed[..at].trim().to_string();
            let version = trimmed[at + 1..].split_whitespace().next().map(String::from);
            if !name.is_empty() {
                out.push(InstalledPackage {
                    name,
                    version,
                    size_bytes: None,
                    manager: "bun".into(),
                });
            }
        }
    }
    out
}

async fn run_deno_modules() -> Vec<InstalledPackage> {
    // Deno's global tool list comes from `deno info --json` (unstable) or the
    // `~/.deno/bin` directory. We surface the Deno version as the single entry
    // to keep the list non-empty without depending on unstable flags.
    let version = run("deno", &["--version"]).await.unwrap_or_default();
    let v = version
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .map(String::from);
    vec![InstalledPackage {
        name: "deno".into(),
        version: v,
        size_bytes: None,
                    manager: "deno".into(),
    }]
}

/// `gem list` outputs lines like `rake (13.0.6)` or, for gems with multiple
/// versions, `rake (13.0.6, 12.3.3)`. We keep the highest version.
async fn run_gem_list() -> Vec<InstalledPackage> {
    let body = match run("gem", &["list", "--local"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with("Gems") {
            continue;
        }
        // Format: "name (1.2.3)" or "name (1.2.3, 1.1.0)".
        if let Some(paren) = trimmed.rfind('(') {
            let name = trimmed[..paren].trim().to_string();
            let versions_part = trimmed[paren + 1..].trim_end_matches(')');
            // Pick the first (highest) listed version.
            let version = versions_part
                .split(',')
                .next()
                .map(|v| v.trim().to_string());
            if !name.is_empty() {
                out.push(InstalledPackage {
                    name,
                    version,
                    size_bytes: None,
                    manager: "gem".into(),
                });
            }
        }
    }
    out
}

/// `composer global show` lists globally-installed Composer packages.
/// Output format: "vendor/name  1.2.3" (name and version separated by spaces).
async fn run_composer_global() -> Vec<InstalledPackage> {
    let body = match run("composer", &["global", "show"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in body.lines() {
        let mut parts = line.split_whitespace();
        let (Some(name), Some(version)) = (parts.next(), parts.next()) else {
            continue;
        };
        // Skip header/noise lines.
        if name.contains(':') || version.contains(':') {
            continue;
        }
        out.push(InstalledPackage {
            name: name.to_string(),
            version: Some(version.to_string()),
            size_bytes: None,
                    manager: "composer".into(),
        });
    }
    out
}

/// `brew list --versions` outputs "name version" per formula, one per line.
/// This includes both formulae and casks; we label them all as `brew`.
async fn run_brew_list() -> Vec<InstalledPackage> {
    let body = match run("brew", &["list", "--versions"]).await {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in body.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let version = parts.next().map(String::from);
        out.push(InstalledPackage {
            name: name.to_string(),
            version,
            size_bytes: None,
                    manager: "brew".into(),
        });
    }
    out
}
