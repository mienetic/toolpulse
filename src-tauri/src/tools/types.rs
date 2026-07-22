//! Shared data types for Toolpulse.
//!
//! These structs are serialized to JSON and sent across the Tauri IPC bridge,
//! so they double as the contract with the TypeScript frontend (`src/types.ts`).

use serde::{Deserialize, Serialize};

/// A single binary installation of a tool discovered on the machine.
///
/// Tools like Node or Python often have several copies installed via different
/// managers (Homebrew, nvm, pyenv, system). We surface each one so the user
/// can pick which to track as the "default".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstallation {
    /// Absolute path to the binary.
    pub path: String,
    /// Version reported by `<binary> --version`, if it could be parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// How this copy was installed: `"system"`, `"homebrew"`, `"nvm"`,
    /// `"pyenv"`, `"asdf"`, `"volta"`, `"unknown"`.
    pub source: InstallationSource,
    /// `true` when this is the copy that resolves first on PATH (i.e. the
    /// active default unless the user overrides it).
    pub is_active: bool,
}

/// Where an installation came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationSource {
    System,
    Homebrew,
    Nvm,
    Pyenv,
    Asdf,
    Volta,
    Conda,
    Rustup,
    Unknown,
}

impl InstallationSource {
    pub fn label(self) -> &'static str {
        match self {
            InstallationSource::System => "system",
            InstallationSource::Homebrew => "homebrew",
            InstallationSource::Nvm => "nvm",
            InstallationSource::Pyenv => "pyenv",
            InstallationSource::Asdf => "asdf",
            InstallationSource::Volta => "volta",
            InstallationSource::Conda => "conda",
            InstallationSource::Rustup => "rustup",
            InstallationSource::Unknown => "unknown",
        }
    }
}

/// Result of scanning a single tool on the local machine.
///
/// This is the primary payload returned to the frontend. Every field is
/// optional except `name` so that a missing tool can still be represented
/// instead of failing the whole scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    /// Stable identifier, e.g. `"node"`, `"rust"`. Used as a key everywhere.
    pub name: String,
    /// Human-friendly label shown in the UI, e.g. `"Node.js"`.
    pub display_name: String,
    /// Category label used for grouping/filtering in the dashboard.
    pub category: ToolCategory,
    /// Emoji used as a lightweight icon (no asset pipeline needed).
    pub icon: String,
    /// Accent color (hex) used for the card header gradient.
    pub color: String,
    /// Version of the active (or user-selected) installation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    /// Latest published version fetched from the network, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// `true` when `installed_version < latest_version`.
    pub is_outdated: bool,
    /// Path of the active/selected installation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Every installation discovered on the machine (multi-version support).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installations: Vec<ToolInstallation>,
    /// Unix timestamp (seconds) of when this check ran.
    pub checked_at: i64,
    /// Populated when the tool could not be detected or the check failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolStatus {
    /// Mark this tool as not installed and record the reason.
    pub fn missing(definition: &ToolDefinition, reason: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            name: definition.name.clone(),
            display_name: definition.display_name.clone(),
            category: definition.category,
            icon: definition.icon.clone(),
            color: definition.color.clone(),
            installed_version: None,
            latest_version: None,
            is_outdated: false,
            path: None,
            installations: Vec::new(),
            checked_at: now,
            error: Some(reason.into()),
        }
    }
}

/// High-level grouping for dashboard filters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// JavaScript / TypeScript runtimes & package managers.
    JavaScript,
    /// Python runtime & tooling.
    Python,
    /// Rust toolchain.
    Rust,
    /// Systems languages (Zig, Go, Swift).
    Systems,
    /// JVM / .NET / other runtimes.
    Runtime,
    /// Container / infra tooling.
    Infra,
    /// System package managers (Homebrew, apt, etc.).
    PackageManager,
    /// Version managers (nvm, pyenv, asdf).
    VersionManager,
}

/// Declarative definition of a tool — how to detect and parse its version.
///
/// The registry is a static slice of these; user-defined tools extend it at
/// runtime via the same shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub display_name: String,
    pub category: ToolCategory,
    pub icon: String,
    pub color: String,
    /// Executable name resolved on PATH, e.g. `"node"` or `"python3"`.
    pub binary: String,
    /// Arguments appended to `binary`, e.g. `["--version"]`.
    pub args: Vec<String>,
    /// Strategy used to extract a version string from stdout.
    pub parser: VersionParser,
    /// How to look up the latest published version.
    pub latest: LatestSource,
    /// Optional command used to enumerate globally installed packages.
    #[serde(default)]
    pub packages: Option<PackageSource>,
    /// If `true`, the scanner probes version managers (nvm/pyenv/asdf/volta)
    /// for additional copies of this binary beyond the one on PATH.
    #[serde(default)]
    pub detect_versions: bool,
    /// Commands for install / uninstall / upgrade. Absent means the tool
    /// cannot be managed from within Toolpulse (e.g. system Python).
    #[serde(default)]
    pub actions: Option<ToolActions>,
}

/// Pluggable version-string extractor.
///
/// Keeping these as an enum (rather than raw regex) makes the registry
/// declarative and avoids embedding untrusted regex from user config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VersionParser {
    /// Match the first `\d+(\.\d+){1,3}` token in the output.
    FirstNumeric,
    /// Strip a leading prefix (`v`, `go`, `python `), then take the first
    /// numeric token.
    StripPrefix { prefix: String },
}

/// Source of truth for the "latest" version of a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LatestSource {
    /// Static version — no network lookup. Useful for tools with no public API.
    None,
    /// JSON endpoint + JSON pointer path, e.g. `["0","version"]`.
    Json { url: String, pointer: Vec<String> },
    /// GitHub releases API (`repos/{owner}/{repo}/releases/latest`).
    GitHub { repo: String },
}

/// How to enumerate globally installed packages for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackageSource {
    /// `npm ls -g --depth=0 --json`
    NpmGlobal,
    /// `cargo install --list` (plain text, parsed).
    CargoInstall,
    /// `pip list --format=json`
    PipList,
    /// `go list -m all` (best effort).
    GoModules,
    /// `bun pm ls -g`
    BunGlobal,
    /// `gem list` (Ruby gems, local scope only).
    GemList,
    /// `composer global show` (PHP Composer global packages).
    ComposerGlobal,
    /// `brew list --versions` (Homebrew formulae).
    BrewList,
    /// `deno info --json` (parsed for cached modules).
    DenoModules,
}

/// How to install / uninstall / upgrade a tool itself.
///
/// Each variant carries the exact argv the runner will execute. Keeping the
/// commands declarative (rather than branching on tool name in code) means
/// adding a new tool is purely data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolActions {
    /// Command to install the tool, e.g. `["brew", "install", "node"]`.
    #[serde(default)]
    pub install: Vec<String>,
    /// Command to uninstall, e.g. `["brew", "uninstall", "node"]`.
    #[serde(default)]
    pub uninstall: Vec<String>,
    /// Command to upgrade to the latest, e.g. `["brew", "upgrade", "node"]`.
    #[serde(default)]
    pub upgrade: Vec<String>,
}

impl ToolActions {
    pub fn is_empty(&self) -> bool {
        self.install.is_empty() && self.uninstall.is_empty() && self.upgrade.is_empty()
    }
}

/// Which operation the user requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Install,
    Uninstall,
    Upgrade,
}

/// One line of terminal output emitted during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalLine {
    /// A plain stdout/stderr line.
    Output { text: String, stream: Stream },
    /// A human-readable status message (e.g. "Running brew install node…").
    Status { text: String },
    /// The run completed.
    Done { success: bool, message: String },
}

/// Which output stream a line came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// A single installed package reported by `list_packages`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Source manager, e.g. `"npm"`, `"cargo"`, `"brew"`.
    pub manager: String,
    /// On-disk size in bytes, when measurable. `None` when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// A point-in-time snapshot stored in the history DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    pub is_outdated: bool,
    pub checked_at: i64,
}

/// Aggregated counts for the dashboard summary cards.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total: usize,
    pub installed: usize,
    pub outdated: usize,
    pub missing: usize,
}

// --- Project scanning -------------------------------------------------------

/// Which ecosystem a discovered project belongs to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProjectEcosystem {
    Node,
    Python,
    Rust,
    Go,
    Ruby,
    Php,
    Java,
    Dotnet,
}

impl ProjectEcosystem {
    pub fn label(self) -> &'static str {
        match self {
            ProjectEcosystem::Node => "Node.js",
            ProjectEcosystem::Python => "Python",
            ProjectEcosystem::Rust => "Rust",
            ProjectEcosystem::Go => "Go",
            ProjectEcosystem::Ruby => "Ruby",
            ProjectEcosystem::Php => "PHP",
            ProjectEcosystem::Java => "Java",
            ProjectEcosystem::Dotnet => ".NET",
        }
    }
}

/// A dependency declared by a project's manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDependency {
    pub name: String,
    /// Declared version spec, e.g. `"^4.2.0"`, `"*"`, `"1.2"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `true` when the latest published version is newer than the declared one.
    /// `None` when we couldn't determine it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_outdated: Option<bool>,
    /// Latest published version, when checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
}

/// A discovered project on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredProject {
    /// Absolute path to the project directory.
    pub path: String,
    /// Project name, derived from the directory or manifest.
    pub name: String,
    pub ecosystem: ProjectEcosystem,
    /// Number of declared dependencies.
    pub dependency_count: usize,
    /// How many are outdated, after checking. `None` if not checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outdated_count: Option<usize>,
    /// On-disk size of the project directory in bytes.
    pub size_bytes: u64,
    /// Absolute path of the manifest file that identified the project.
    pub manifest: String,
    /// `true` when the directory looks like a real project (has source dirs,
    /// multiple files, etc.) rather than a stray manifest file. The UI hides
    /// non-projects by default.
    pub is_real_project: bool,
}

/// Aggregated counts for the project scanner.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectScanSummary {
    pub total: usize,
    pub by_ecosystem: std::collections::HashMap<String, usize>,
    pub total_size_bytes: u64,
}

// --- Standalone source-file scanning ----------------------------------------

/// A programming language we can detect standalone source files for.
///
/// Each variant is explicitly renamed to match the frontend's string values
/// (e.g. `javascript`, `typescript`) rather than relying on `rename_all`,
/// which would produce `java_script` / `type_script`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SourceLanguage {
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "typescript")]
    TypeScript,
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "go")]
    Go,
    #[serde(rename = "ruby")]
    Ruby,
    #[serde(rename = "php")]
    Php,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "swift")]
    Swift,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "cpp")]
    Cpp,
}

impl SourceLanguage {
    pub fn label(self) -> &'static str {
        match self {
            SourceLanguage::JavaScript => "JavaScript",
            SourceLanguage::TypeScript => "TypeScript",
            SourceLanguage::Python => "Python",
            SourceLanguage::Rust => "Rust",
            SourceLanguage::Go => "Go",
            SourceLanguage::Ruby => "Ruby",
            SourceLanguage::Php => "PHP",
            SourceLanguage::Java => "Java",
            SourceLanguage::Swift => "Swift",
            SourceLanguage::C => "C",
            SourceLanguage::Cpp => "C++",
        }
    }

    /// File extensions that identify this language.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            SourceLanguage::JavaScript => &["js", "jsx", "mjs", "cjs"],
            SourceLanguage::TypeScript => &["ts", "tsx"],
            SourceLanguage::Python => &["py"],
            SourceLanguage::Rust => &["rs"],
            SourceLanguage::Go => &["go"],
            SourceLanguage::Ruby => &["rb"],
            SourceLanguage::Php => &["php"],
            SourceLanguage::Java => &["java"],
            SourceLanguage::Swift => &["swift"],
            SourceLanguage::C => &["c", "h"],
            SourceLanguage::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx"],
        }
    }

    /// All supported languages, in display order.
    pub fn all() -> &'static [SourceLanguage] {
        &[
            SourceLanguage::JavaScript,
            SourceLanguage::TypeScript,
            SourceLanguage::Python,
            SourceLanguage::Rust,
            SourceLanguage::Go,
            SourceLanguage::Ruby,
            SourceLanguage::Php,
            SourceLanguage::Java,
            SourceLanguage::Swift,
            SourceLanguage::C,
            SourceLanguage::Cpp,
        ]
    }
}

/// A standalone source file found outside any project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub name: String,
    pub language: SourceLanguage,
    pub size_bytes: u64,
}
