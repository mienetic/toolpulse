//! Discovers projects on disk by walking a directory tree for manifests.
//!
//! Each ecosystem has a signature file (e.g. `package.json`, `Cargo.toml`).
//! We walk the tree breadth-first, skipping heavy/irrelevant directories
//! (`node_modules`, `.git`, `target`, build caches) so a `$HOME` scan stays
//! fast. When a manifest is found, we stop descending into that directory for
//! the same ecosystem (nested duplicates are noise) but keep looking for
//! *other* ecosystems.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::fs;
use tokio::process::Command;

use super::types::*;

/// Directory basenames to never descend into. These are either dependency
/// caches (huge, slow, redundant) or VCS/build artifacts.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    "build",
    "dist",
    ".next",
    ".nuxt",
    ".cache",
    ".gradle",
    ".m2",
    "venv",
    ".venv",
    "env",
    "__pycache__",
    ".tox",
    ".pytest_cache",
    "vendor",
    ".cargo",
    ".rustup",
    "Pods",
    "DerivedData",
    ".deno",
    ".bun",
    "coverage",
    ".turbo",
];

/// Manifest signatures per ecosystem: `(filename, ecosystem)`.
const SIGNATURES: &[(&str, ProjectEcosystem)] = &[
    ("package.json", ProjectEcosystem::Node),
    ("requirements.txt", ProjectEcosystem::Python),
    ("pyproject.toml", ProjectEcosystem::Python),
    ("Pipfile", ProjectEcosystem::Python),
    ("Cargo.toml", ProjectEcosystem::Rust),
    ("go.mod", ProjectEcosystem::Go),
    ("Gemfile", ProjectEcosystem::Ruby),
    ("composer.json", ProjectEcosystem::Php),
    ("pom.xml", ProjectEcosystem::Java),
    ("build.gradle", ProjectEcosystem::Java),
    ("build.gradle.kts", ProjectEcosystem::Java),
];

/// Walk `root` and collect every project found.
pub async fn scan(root: &Path) -> Vec<DiscoveredProject> {
    scan_roots(&[root.to_path_buf()], &|_| {}, &|_| {}).await
}

/// Scan multiple roots (machine-wide) and report progress progressively.
///
/// `on_dir` is called for every directory visited (for a live "scanning X"
/// indicator), and `on_project` is called the moment each project is found so
/// the UI can append it without waiting for the whole scan to finish.
pub async fn scan_roots(
    roots: &[PathBuf],
    on_dir: &(dyn Fn(&str) + Send + Sync),
    on_project: &(dyn Fn(&DiscoveredProject) + Send + Sync),
) -> Vec<DiscoveredProject> {
    let mut found: Vec<DiscoveredProject> = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    // BFS queue (FIFO) so we cover breadth before depth — this ensures
    // sibling directories like ~/Documents get visited even when an earlier
    // directory (e.g. ~/projects) has a deep subtree.
    use std::collections::VecDeque;
    let mut queue: VecDeque<PathBuf> = roots.iter().cloned().collect();

    while let Some(dir) = queue.pop_front() {
        if visited.contains(&dir) {
            continue;
        }
        visited.insert(dir.clone());
        on_dir(&dir.to_string_lossy());

        let entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Track which ecosystems we already matched here, so we don't double-
        // count (e.g. requirements.txt + pyproject.toml in the same dir).
        let mut matched: HashSet<ProjectEcosystem> = HashSet::new();
        let mut child_dirs: Vec<PathBuf> = Vec::new();

        let mut entry_iter = entries;
        while let Ok(Some(entry)) = tokio_stream_next(&mut entry_iter).await {
            let file_type = match entry.file_type().await {
                Ok(t) => t,
                Err(_) => continue,
            };
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if file_type.is_dir() {
                if !SKIP_DIRS.contains(&name_str.as_ref()) && !name_str.starts_with('.') {
                    child_dirs.push(entry.path());
                }
                continue;
            }

            // Check if this file is a known manifest.
            if let Some((_, eco)) = SIGNATURES.iter().find(|(sig, _)| *sig == name_str) {
                if matched.insert(*eco) {
                    if let Some(proj) = build_project(&dir, &entry.path(), *eco).await {
                        on_project(&proj);
                        found.push(proj);
                    }
                }
            }
        }

        // Descend into children (reversed so earlier dirs are processed first).
        // Enqueue children at the back (FIFO) for true breadth-first order.
        queue.extend(child_dirs);

        // Cap the result set so a runaway scan doesn't OOM.
        if found.len() >= 2000 {
            break;
        }
    }

    found
}

/// Tokio's `read_dir` returns a stream-like async iterator without the
/// `tokio-stream` crate. We poll it manually here.
async fn tokio_stream_next(
    entries: &mut tokio::fs::ReadDir,
) -> Result<Option<tokio::fs::DirEntry>, std::io::Error> {
    entries.next_entry().await
}

/// Build a `DiscoveredProject` from a matched manifest.
async fn build_project(
    dir: &Path,
    manifest: &Path,
    eco: ProjectEcosystem,
) -> Option<DiscoveredProject> {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());
    let size = du_dir(dir).await.unwrap_or(0);
    let deps = parse_deps_detail(manifest, eco).await;
    let is_real = is_real_project(dir, eco, &deps).await;

    Some(DiscoveredProject {
        path: dir.to_string_lossy().to_string(),
        name,
        ecosystem: eco,
        dependency_count: deps.len(),
        outdated_count: None,
        size_bytes: size,
        manifest: manifest.to_string_lossy().to_string(),
        is_real_project: is_real,
    })
}

/// A directory is a real project whenever it contains a recognized manifest
/// file (package.json, Cargo.toml, go.mod, …). The scanner only calls this
/// for directories where a manifest was already found, so it always returns
/// `true` — the "loose manifest" filter is handled by the file scanner
/// (which skips directories containing a manifest).
async fn is_real_project(_dir: &Path, _eco: ProjectEcosystem, _deps: &[ProjectDependency]) -> bool {
    true
}

/// Parse a manifest into detailed dependency entries.
pub async fn parse_deps_detail(
    manifest: &Path,
    eco: ProjectEcosystem,
) -> Vec<ProjectDependency> {
    let body = match fs::read_to_string(manifest).await {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    match eco {
        ProjectEcosystem::Node => parse_node_deps(&body),
        ProjectEcosystem::Python => parse_python_deps(&body, manifest),
        ProjectEcosystem::Rust => parse_cargo_deps(&body),
        ProjectEcosystem::Go => parse_go_deps(&body),
        ProjectEcosystem::Ruby => parse_gemfile_deps(&body),
        ProjectEcosystem::Php => parse_composer_deps(&body),
        ProjectEcosystem::Java => Vec::new(), // gradle/maven need tooling
        ProjectEcosystem::Dotnet => Vec::new(),
    }
}

fn parse_node_deps(body: &str) -> Vec<ProjectDependency> {
    let value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = value.get(section).and_then(|v| v.as_object()) {
            for (name, ver) in obj {
                deps.push(ProjectDependency {
                    name: name.clone(),
                    version: ver.as_str().map(String::from),
                    is_outdated: None,
                    latest: None,
                });
            }
        }
    }
    deps
}

fn parse_python_deps(body: &str, manifest: &Path) -> Vec<ProjectDependency> {
    let fname = manifest.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match fname {
        "requirements.txt" => body
            .lines()
            .filter_map(|l| {
                let l = l.split('#').next()?.trim();
                if l.is_empty() {
                    return None;
                }
                // "package==1.2.3" or "package>=1.0" or "package"
                let mut parts = l.splitn(2, |c: char| matches!(c, '=' | '>' | '<' | '~' | '!'));
                let name = parts.next()?.trim();
                let ver = parts.next().map(|s| s.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.').to_string());
                Some(ProjectDependency {
                    name: name.to_string(),
                    version: ver,
                    is_outdated: None,
                    latest: None,
                })
            })
            .collect(),
        "Pipfile" => body
            .lines()
            .filter(|l| l.contains("=") && (l.trim_start().starts_with(|c: char| c.is_alphanumeric())))
            .filter_map(|l| {
                let l = l.trim();
                let name = l.split('=').next()?.trim().trim_matches('"');
                if name.is_empty() || name == "source" {
                    return None;
                }
                Some(ProjectDependency {
                    name: name.to_string(),
                    version: None,
                    is_outdated: None,
                    latest: None,
                })
            })
            .collect(),
        // pyproject.toml — best-effort line parse for [project] dependencies.
        _ => body
            .lines()
            .skip_while(|l| !l.contains("dependencies"))
            .skip(1)
            .take_while(|l| !l.trim().is_empty() && !l.starts_with('['))
            .filter_map(|l| {
                let l = l.trim().trim_end_matches(',').trim_matches('"').trim_matches('\'');
                if l.is_empty() {
                    return None;
                }
                let name = l.split(|c: char| matches!(c, '<' | '>' | '=' | '~' | '!' | ' ')).next()?;
                Some(ProjectDependency {
                    name: name.to_string(),
                    version: None,
                    is_outdated: None,
                    latest: None,
                })
            })
            .collect(),
    }
}

fn parse_cargo_deps(body: &str) -> Vec<ProjectDependency> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed.starts_with("[dependencies") || trimmed.starts_with("[dev-dependencies");
            continue;
        }
        if !in_deps {
            continue;
        }
        // `name = "version"` or `name = { version = "..." }` or `name = { path = "..." }`
        if let Some(eq) = trimmed.find('=') {
            let name = trimmed[..eq].trim();
            if name.is_empty() || name.starts_with('#') {
                continue;
            }
            let rest = trimmed[eq + 1..].trim();
            let version = if rest.starts_with('"') {
                rest.trim_matches('"').to_string().into()
            } else if rest.contains("version") {
                rest.split("version")
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .map(String::from)
                    .into()
            } else {
                None
            };
            deps.push(ProjectDependency {
                name: name.to_string(),
                version,
                is_outdated: None,
                latest: None,
            });
        }
    }
    deps
}

fn parse_go_deps(body: &str) -> Vec<ProjectDependency> {
    body.lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("require ")?.trim_start_matches('(').trim();
            if l.is_empty() {
                return None;
            }
            let mut parts = l.split_whitespace();
            let name = parts.next()?;
            let version = parts.next().map(String::from);
            Some(ProjectDependency {
                name: name.to_string(),
                version,
                is_outdated: None,
                latest: None,
            })
        })
        .collect()
}

fn parse_gemfile_deps(body: &str) -> Vec<ProjectDependency> {
    body.lines()
        .filter_map(|l| {
            let l = l.trim();
            let l = l.strip_prefix("gem ")?;
            let l = l.trim_start_matches(|c: char| c == '\'' || c == '"');
            let name = l.split(|c: char| c == '\'' || c == '"').next()?;
            if name.is_empty() {
                return None;
            }
            Some(ProjectDependency {
                name: name.to_string(),
                version: None,
                is_outdated: None,
                latest: None,
            })
        })
        .collect()
}

fn parse_composer_deps(body: &str) -> Vec<ProjectDependency> {
    let value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for section in ["require", "require-dev"] {
        if let Some(obj) = value.get(section).and_then(|v| v.as_object()) {
            for (name, ver) in obj {
                deps.push(ProjectDependency {
                    name: name.clone(),
                    version: ver.as_str().map(String::from),
                    is_outdated: None,
                    latest: None,
                });
            }
        }
    }
    deps
}

/// `du -sk <dir>` → bytes. Falls back to 0 on error.
async fn du_dir(dir: &Path) -> Option<u64> {
    let out = Command::new("du")
        .args(["-sk", &dir.to_string_lossy()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let kb: u64 = stdout.split_whitespace().next()?.parse().ok()?;
    Some(kb.saturating_mul(1024))
}

/// Check a list of Node/npm dependencies against the npm registry to flag
/// outdated ones. Other ecosystems are skipped (no public "latest" API that's
/// cheap to call per-dep without their tooling).
pub async fn check_outdated(
    deps: &mut [ProjectDependency],
    http: &reqwest::Client,
    ecosystem: ProjectEcosystem,
) {
    if ecosystem != ProjectEcosystem::Node {
        // Only npm has a trivial JSON endpoint per package.
        for d in deps.iter_mut() {
            d.is_outdated = None;
        }
        return;
    }
    // For each dep, fetch the latest version from the npm registry concurrently.
    let futures: Vec<_> = deps
        .iter()
        .map(|d| {
            let name = d.name.clone();
            let http = http.clone();
            async move {
                let url = format!("https://registry.npmjs.org/{}/latest", name);
                let resp = http
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(8))
                    .send()
                    .await
                    .ok()?;
                if !resp.status().is_success() {
                    return None;
                }
                let value: Value = resp.json().await.ok()?;
                value
                    .get("version")
                    .and_then(|x| x.as_str())
                    .map(String::from)
            }
        })
        .collect();
    let latests = futures::future::join_all(futures).await;
    for (dep, latest) in deps.iter_mut().zip(latests) {
        if let Some(latest) = latest {
            let declared = dep.version.as_deref().unwrap_or("");
            // Strip semver operators (^, ~, >=) for the comparison.
            let declared_clean = declared.trim_start_matches(|c: char| !c.is_ascii_digit());
            let is_old = !declared_clean.is_empty()
                && crate::tools::is_outdated(declared_clean, &latest);
            dep.latest = Some(latest);
            dep.is_outdated = Some(is_old);
        } else {
            dep.is_outdated = None;
        }
    }
}
