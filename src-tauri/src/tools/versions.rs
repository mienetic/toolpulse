//! Multi-version installation detection.
//!
//! Tools like Node or Python are frequently installed several times via
//! different managers (Homebrew, nvm, pyenv, asdf, system). This module finds
//! every copy on the machine so the user can choose which one to track.
//!
//! Strategy:
//!   1. `which -a <binary>` lists every PATH match, in priority order.
//!   2. For each, we infer the source from the path (e.g. `/nvm/` → nvm) and
//!      run `<binary> --version` to capture its version.
//!   3. The first PATH entry is marked `is_active`.
//!
//! Version managers are *not* invoked directly (e.g. `nvm ls`) because they
//! are often shell functions and not reliably invokable from a subprocess.
//! Path-based inference is more robust.

use std::process::Stdio;

use tokio::process::Command;

use super::checker::parse_version;
use super::types::*;

/// Find every installation of `def.binary` on the machine.
///
/// Returns an empty vec if none are found. The first entry is the active one
/// (resolves first on PATH).
pub async fn detect_all(def: &ToolDefinition) -> Vec<ToolInstallation> {
    let paths = which_all(&def.binary).await;
    if paths.is_empty() {
        return Vec::new();
    }

    let mut installs = Vec::with_capacity(paths.len());
    for (idx, path) in paths.iter().enumerate() {
        let source = infer_source(path);
        let version = run_version(path, &def.args, &def.parser).await;
        installs.push(ToolInstallation {
            path: path.clone(),
            version,
            source,
            is_active: idx == 0,
        });
    }
    installs
}

/// Run `which -a <binary>` and split the output into absolute paths.
async fn which_all(binary: &str) -> Vec<String> {
    let out = Command::new("which")
        .arg("-a")
        .arg(binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;
    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Infer the installation source from the binary's path.
///
/// Public so the checker can label a user-pinned path that wasn't found by
/// the normal `which -a` enumeration.
pub fn infer_source_public(path: &str) -> InstallationSource {
    infer_source(path)
}

/// Infer the installation source from the binary's path.
fn infer_source(path: &str) -> InstallationSource {
    let p = path.to_lowercase();
    if p.contains("/nvm/") || p.contains("/.nvm/") || p.contains("/nvm/versions/") {
        InstallationSource::Nvm
    } else if p.contains("/pyenv/") || p.contains("/.pyenv/") || p.contains("/pyenv/versions/") {
        InstallationSource::Pyenv
    } else if p.contains("/asdf/") || p.contains("/.asdf/") {
        InstallationSource::Asdf
    } else if p.contains("/volta/") || p.contains("/.volta/") {
        InstallationSource::Volta
    } else if p.contains("/.rustup/") || p.contains("/rustup/") {
        InstallationSource::Rustup
    } else if p.contains("/miniconda") || p.contains("/anaconda") || p.contains("/conda/")
    {
        InstallationSource::Conda
    } else if p.contains("/homebrew/") || p.contains("/linuxbrew/") {
        InstallationSource::Homebrew
    } else if p.contains("/usr/bin/") || p.contains("/bin/") || p.contains("/usr/local/bin/") {
        InstallationSource::System
    } else {
        InstallationSource::Unknown
    }
}

/// Execute `<path> <args>` and extract a version string.
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
