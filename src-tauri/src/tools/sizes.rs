//! On-disk size measurement for installed packages.
//!
//! Strategy (mixed): try the package manager's own size report first (fast,
//! already computed), then fall back to `du -sk` on the package's install
//! directory. Both are async and run concurrently.
//!
//! Not every manager exposes a size, and some packages (e.g. pip in a shared
//! site-packages) can't be attributed to a single directory — those simply
//! report `None`, and the UI shows "—".

use std::process::Stdio;

use tokio::process::Command;

use super::types::*;

/// Fill in `size_bytes` for every package, in place, using the manager's
/// convention to locate the install directory.
pub async fn measure_sizes(packages: &mut [InstalledPackage]) {
    // Run all measurements concurrently; we collect futures then await.
    let futures: Vec<_> = packages
        .iter()
        .map(|p| measure_one(p))
        .collect();
    let results = futures::future::join_all(futures).await;
    for (pkg, size) in packages.iter_mut().zip(results) {
        pkg.size_bytes = size;
    }
}

async fn measure_one(pkg: &InstalledPackage) -> Option<u64> {
    match pkg.manager.as_str() {
        "npm" => npm_size(&pkg.name).await,
        "brew" => brew_size(&pkg.name).await,
        "cargo" => cargo_size(&pkg.name).await,
        "pip" => pip_size(&pkg.name).await,
        "gem" => gem_size(&pkg.name).await,
        "composer" => composer_size(&pkg.name).await,
        "go" => None, // go modules aren't a global install; no single dir
        "bun" => bun_size(&pkg.name).await,
        "deno" => None, // deno exposes no per-tool dir
        _ => None,
    }
}

// --- Per-manager locators ---------------------------------------------------

/// `npm root -g` gives the global `node_modules`; the package is a subdir.
async fn npm_size(name: &str) -> Option<u64> {
    let root = run_capture("npm", &["root", "-g"]).await?;
    let root = root.trim();
    // Scope: packages like '@scope/pkg' live under `@scope/pkg`.
    let path = format!("{root}/{name}");
    du_bytes(&path).await
}

/// Homebrew Cellar: `/opt/homebrew/Cellar/<name>/<version>` (or `/usr/local/...`).
async fn brew_size(name: &str) -> Option<u64> {
    // `brew info --json=v2` includes the installed keg size, but it's heavy;
    // a direct du on the Cellar dir is faster and good enough here.
    for prefix in ["/opt/homebrew/Cellar", "/usr/local/Cellar"] {
        let path = format!("{prefix}/{name}");
        if let Some(bytes) = du_bytes(&path).await {
            return Some(bytes);
        }
    }
    None
}

/// Cargo installs live under `~/.cargo/registry/src/<index>/<name>-<ver>`.
/// We glob the name prefix since the version is unknown here.
async fn cargo_size(name: &str) -> Option<u64> {
    let home = std::env::var("HOME").ok()?;
    let glob = format!("{home}/.cargo/registry/src/*/{name}-*");
    du_bytes_glob(&glob).await
}

/// pip packages share a `site-packages` dir; each is either `<name>/` or
/// `<name>-<version>.dist-info`. We measure the package dir, trying both the
/// raw and dash-normalized names.
async fn pip_size(name: &str) -> Option<u64> {
    let site = run_capture(
        "python3",
        &["-c", "import site,sys;print(site.getsitepackages()[0])"],
    )
    .await?;
    let site = site.trim();
    let path = format!("{site}/{name}");
    if let Some(bytes) = du_bytes(&path).await {
        return Some(bytes);
    }
    // pip normalizes dashes to underscores in directory names.
    let path2 = format!("{site}/{}", name.replace('-', "_"));
    du_bytes(&path2).await
}

/// `gem environment` lists the gem home; gems live under `gems/<name>-<ver>`.
async fn gem_size(name: &str) -> Option<u64> {
    let out = run_capture("gem", &["environment"]).await?;
    // Find a "GEM PATHS" entry pointing at an installable dir.
    let home = std::env::var("HOME").ok()?;
    let glob = format!("{home}/.gem/ruby/*/gems/{name}-*");
    du_bytes_glob(&glob).await
}

/// Composer global packages live under `~/.composer/vendor/<name>`.
async fn composer_size(name: &str) -> Option<u64> {
    let home = std::env::var("HOME").ok()?;
    // Composer uses vendor-name/package; the first segment is the vendor.
    let path = format!("{home}/.composer/vendor/{name}");
    if let Some(bytes) = du_bytes(&path).await {
        return Some(bytes);
    }
    // Fall back to a glob in case the version is appended.
    let glob = format!("{home}/.composer/vendor/{name}");
    du_bytes_glob(&glob).await
}

/// Bun global packages live under `~/.bun/install/global/node_modules/<name>`.
async fn bun_size(name: &str) -> Option<u64> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{home}/.bun/install/global/node_modules/{name}");
    du_bytes(&path).await
}

// --- Helpers ----------------------------------------------------------------

/// Run a command and return its trimmed stdout, or `None` on failure.
async fn run_capture(cmd: &str, args: &[&str]) -> Option<String> {
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

/// `du -sk <path>` → kilobytes. Multiply by 1024 for bytes.
async fn du_bytes(path: &str) -> Option<u64> {
    let out = Command::new("du")
        .args(["-sk", path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_du(&String::from_utf8_lossy(&out.stdout))
}

/// `du -sk <glob>` — shells out so the glob expands. Returns the sum across
/// all matched entries.
async fn du_bytes_glob(glob: &str) -> Option<u64> {
    let out = Command::new("sh")
        .args(["-c", &format!("du -sk {glob} 2>/dev/null")])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !out.status.success() && out.stdout.is_empty() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Sum every "12345\t/path" line.
    let mut total: u64 = 0;
    let mut found = false;
    for line in stdout.lines() {
        if let Some(rest) = parse_du_line(line) {
            total = total.saturating_add(rest);
            found = true;
        }
    }
    if found { Some(total) } else { None }
}

/// Parse `du -sk` output: first whitespace-delimited token is KB.
fn parse_du(stdout: &str) -> Option<u64> {
    stdout.lines().next().and_then(parse_du_line)
}

fn parse_du_line(line: &str) -> Option<u64> {
    let kb: u64 = line.split_whitespace().next()?.parse().ok()?;
    Some(kb.saturating_mul(1024))
}
