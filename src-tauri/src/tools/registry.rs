//! Static registry of supported development tools.
//!
//! Each entry is fully declarative: binary, args, parser, latest source, and
//! optional package listing. Adding a new tool is just appending a `ToolDefinition`
//! here — no branching logic required.

use super::types::*;

// --- Action helpers ---------------------------------------------------------
// Each helper builds the `ToolActions` (install / uninstall / upgrade) for a
// given package manager convention, so the registry entries stay one-liners.

fn brew(formula: &str) -> ToolActions {
    ToolActions {
        install: strs(&["brew", "install", formula]),
        uninstall: strs(&["brew", "uninstall", formula]),
        upgrade: strs(&["brew", "upgrade", formula]),
    }
}

fn npm_global(pkg: &str) -> ToolActions {
    ToolActions {
        install: strs(&["npm", "install", "-g", pkg]),
        uninstall: strs(&["npm", "uninstall", "-g", pkg]),
        upgrade: strs(&["npm", "install", "-g", &format!("{pkg}@latest")]),
    }
}

fn rustup_component(name: &str) -> ToolActions {
    ToolActions {
        install: strs(&["rustup", "component", "add", name]),
        uninstall: strs(&["rustup", "component", "remove", name]),
        upgrade: strs(&["rustup", "update"]),
    }
}

/// Turn a slice of `&str` into an owned `Vec<String>`.
fn strs(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

/// The canonical list of tools Toolpulse knows how to inspect.
///
/// Order is preserved for stable display in the UI.
pub fn builtin_tools() -> Vec<ToolDefinition> {
    vec![
        // ---------- JavaScript / TypeScript ----------
        ToolDefinition {
            name: "node".into(),
            display_name: "Node.js".into(),
            category: ToolCategory::JavaScript,
            icon: "🟢".into(),
            color: "#5fa04e".into(),
            binary: "node".into(),
            args: vec!["--version".into()],
            parser: VersionParser::StripPrefix { prefix: "v".into() },
            latest: LatestSource::Json {
                url: "https://nodejs.org/dist/index.json".into(),
                pointer: vec!["0".into(), "version".into()],
            },
            packages: None,
            actions: None,
            detect_versions: true,
        },
        ToolDefinition {
            name: "npm".into(),
            display_name: "npm".into(),
            category: ToolCategory::JavaScript,
            icon: "📦".into(),
            color: "#cb3837".into(),
            binary: "npm".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://registry.npmjs.org/npm/latest".into(),
                pointer: vec!["version".into()],
            },
            packages: Some(PackageSource::NpmGlobal),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "yarn".into(),
            display_name: "Yarn".into(),
            category: ToolCategory::JavaScript,
            icon: "🧶".into(),
            color: "#2c8ebb".into(),
            binary: "yarn".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://registry.yarnpkg.com/yarn/latest".into(),
                pointer: vec!["version".into()],
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "pnpm".into(),
            display_name: "pnpm".into(),
            category: ToolCategory::JavaScript,
            icon: "📦".into(),
            color: "#f69220".into(),
            binary: "pnpm".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://registry.npmjs.org/pnpm/latest".into(),
                pointer: vec!["version".into()],
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "bun".into(),
            display_name: "Bun".into(),
            category: ToolCategory::JavaScript,
            icon: "🥖".into(),
            color: "#fbf0df".into(),
            binary: "bun".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "oven-sh/bun".into(),
            },
            packages: Some(PackageSource::BunGlobal),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "deno".into(),
            display_name: "Deno".into(),
            category: ToolCategory::JavaScript,
            icon: "🦕".into(),
            color: "#70ffaf".into(),
            binary: "deno".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "denoland/deno".into(),
            },
            packages: Some(PackageSource::DenoModules),
            actions: None,
            detect_versions: false,
        },
        // ---------- Python ----------
        ToolDefinition {
            name: "python".into(),
            display_name: "Python".into(),
            category: ToolCategory::Python,
            icon: "🐍".into(),
            color: "#3776ab".into(),
            binary: "python3".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: None,
            actions: None,
            detect_versions: true,
        },
        ToolDefinition {
            name: "pip".into(),
            display_name: "pip".into(),
            category: ToolCategory::Python,
            icon: "📥".into(),
            color: "#3776ab".into(),
            binary: "pip3".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: Some(PackageSource::PipList),
            actions: None,
            detect_versions: false,
        },
        // ---------- Rust ----------
        ToolDefinition {
            name: "rust".into(),
            display_name: "Rust".into(),
            category: ToolCategory::Rust,
            icon: "🦀".into(),
            color: "#dea584".into(),
            binary: "rustc".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://static.rust-lang.org/dist/channel-rust-stable.toml".into(),
                pointer: vec![],
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "cargo".into(),
            display_name: "Cargo".into(),
            category: ToolCategory::Rust,
            icon: "📦".into(),
            color: "#dea584".into(),
            binary: "cargo".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: Some(PackageSource::CargoInstall),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "rustup".into(),
            display_name: "rustup".into(),
            category: ToolCategory::Rust,
            icon: "🔧".into(),
            color: "#dea584".into(),
            binary: "rustup".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "rust-lang/rustup".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        // ---------- Systems languages ----------
        ToolDefinition {
            name: "zig".into(),
            display_name: "Zig".into(),
            category: ToolCategory::Systems,
            icon: "⚡".into(),
            color: "#f7a41d".into(),
            binary: "zig".into(),
            args: vec!["version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://ziglang.org/download/index.json".into(),
                pointer: vec![],
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "go".into(),
            display_name: "Go".into(),
            category: ToolCategory::Systems,
            icon: "🐹".into(),
            color: "#00add8".into(),
            binary: "go".into(),
            args: vec!["version".into()],
            parser: VersionParser::StripPrefix { prefix: "go".into() },
            latest: LatestSource::Json {
                url: "https://go.dev/dl/?mode=json".into(),
                pointer: vec!["0".into(), "version".into()],
            },
            packages: Some(PackageSource::GoModules),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "swift".into(),
            display_name: "Swift".into(),
            category: ToolCategory::Systems,
            icon: "🕊️".into(),
            color: "#f05138".into(),
            binary: "swift".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "swiftlang/swift".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "ruby".into(),
            display_name: "Ruby".into(),
            category: ToolCategory::Systems,
            icon: "💎".into(),
            color: "#cc342d".into(),
            binary: "ruby".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: None,
            actions: None,
            detect_versions: true,
        },
        // ---------- Other runtimes ----------
        ToolDefinition {
            name: "dotnet".into(),
            display_name: ".NET SDK".into(),
            category: ToolCategory::Runtime,
            icon: "🌐".into(),
            color: "#512bd4".into(),
            binary: "dotnet".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "java".into(),
            display_name: "Java".into(),
            category: ToolCategory::Runtime,
            icon: "☕".into(),
            color: "#ed8b00".into(),
            binary: "java".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "php".into(),
            display_name: "PHP".into(),
            category: ToolCategory::Runtime,
            icon: "🐘".into(),
            color: "#777bb4".into(),
            binary: "php".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: None,
            actions: None,
            detect_versions: false,
        },
        // ---------- Infra ----------
        ToolDefinition {
            name: "docker".into(),
            display_name: "Docker".into(),
            category: ToolCategory::Infra,
            icon: "🐳".into(),
            color: "#2496ed".into(),
            binary: "docker".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "moby/moby".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        // ---------- Package managers ----------
        ToolDefinition {
            name: "brew".into(),
            display_name: "Homebrew".into(),
            category: ToolCategory::PackageManager,
            icon: "🍺".into(),
            color: "#fbb040".into(),
            binary: "brew".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "Homebrew/brew".into(),
            },
            packages: Some(PackageSource::BrewList),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "gem".into(),
            display_name: "RubyGems".into(),
            category: ToolCategory::PackageManager,
            icon: "💎".into(),
            color: "#cc342d".into(),
            binary: "gem".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::Json {
                url: "https://rubygems.org/api/v1/versions/rubygems-update/latest.json".into(),
                pointer: vec!["version".into()],
            },
            packages: Some(PackageSource::GemList),
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "composer".into(),
            display_name: "Composer".into(),
            category: ToolCategory::PackageManager,
            icon: "🎵".into(),
            color: "#885630".into(),
            binary: "composer".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::None,
            packages: Some(PackageSource::ComposerGlobal),
            actions: None,
            detect_versions: false,
        },
        // ---------- Version managers ----------
        ToolDefinition {
            name: "nvm".into(),
            display_name: "nvm".into(),
            category: ToolCategory::VersionManager,
            icon: "🔵".into(),
            color: "#5fa04e".into(),
            // nvm is a shell function, not a binary on PATH. We probe it via
            // the `nvm --version` form supported by bash/zsh nvm.
            binary: "nvm".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "nvm-sh/nvm".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "pyenv".into(),
            display_name: "pyenv".into(),
            category: ToolCategory::VersionManager,
            icon: "🐍".into(),
            color: "#3776ab".into(),
            binary: "pyenv".into(),
            args: vec!["--version".into()],
            parser: VersionParser::StripPrefix { prefix: "pyenv".into() },
            latest: LatestSource::GitHub {
                repo: "pyenv/pyenv".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "asdf".into(),
            display_name: "asdf".into(),
            category: ToolCategory::VersionManager,
            icon: "🔺".into(),
            color: "#a8553a".into(),
            binary: "asdf".into(),
            args: vec!["--version".into()],
            parser: VersionParser::StripPrefix { prefix: "v".into() },
            latest: LatestSource::GitHub {
                repo: "asdf-vm/asdf".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
        ToolDefinition {
            name: "volta".into(),
            display_name: "Volta".into(),
            category: ToolCategory::VersionManager,
            icon: "⚡".into(),
            color: "#5fa04e".into(),
            binary: "volta".into(),
            args: vec!["--version".into()],
            parser: VersionParser::FirstNumeric,
            latest: LatestSource::GitHub {
                repo: "volta-cli/volta".into(),
            },
            packages: None,
            actions: None,
            detect_versions: false,
        },
    ]
    .into_iter()
    .map(apply_default_actions)
    .collect()
}

/// Attach install/uninstall/upgrade commands to each tool based on its name.
///
/// Centralizing this here keeps the per-tool declarations free of repeated
/// boilerplate and makes the package-manager conventions easy to audit.
fn apply_default_actions(mut def: ToolDefinition) -> ToolDefinition {
    // Skip if actions were already specified inline.
    if def.actions.is_some() {
        return def;
    }
    let actions = match def.name.as_str() {
        // Homebrew-managed runtimes & languages.
        "node" | "python" | "go" | "ruby" | "zig" | "deno" | "bun" | "swift"
        | "php" | "docker" => Some(brew(def.name.as_str())),
        // Rust toolchain via rustup.
        "rust" | "rustc" => Some(rustup_component("rustc")),
        "cargo" => Some(rustup_component("cargo")),
        "rustup" => Some(ToolActions {
            install: strs(&["curl", "--proto", "=https", "--tlsv1.2", "-sSf",
                "https://sh.rustup.rs", "|", "sh", "-s", "--", "-y"]),
            uninstall: strs(&["rustup", "self", "uninstall", "-y"]),
            upgrade: strs(&["rustup", "update"]),
        }),
        // npm-family package managers install themselves via npm.
        "npm" => Some(npm_global("npm")),
        "yarn" => Some(npm_global("yarn")),
        "pnpm" => Some(npm_global("pnpm")),
        // .NET / Java ship via brew casks or vendor installers; mark as
        // brew-installable for simplicity.
        "dotnet" => Some(brew("dotnet")),
        "java" => Some(ToolActions {
            install: strs(&["brew", "install", "--cask", "temurin"]),
            uninstall: strs(&["brew", "uninstall", "--cask", "temurin"]),
            upgrade: strs(&["brew", "upgrade", "--cask", "temurin"]),
        }),
        // Package managers can update themselves.
        "brew" => Some(ToolActions {
            install: Vec::new(), // already installed if present
            uninstall: Vec::new(),
            upgrade: strs(&["brew", "update"]),
        }),
        "gem" => Some(ToolActions {
            install: strs(&["gem", "update", "--system"]),
            uninstall: Vec::new(),
            upgrade: strs(&["gem", "update", "--system"]),
        }),
        "composer" => Some(brew("composer")),
        // Version managers.
        "nvm" => Some(ToolActions {
            install: strs(&["brew", "install", "nvm"]),
            uninstall: strs(&["brew", "uninstall", "nvm"]),
            upgrade: strs(&["brew", "upgrade", "nvm"]),
        }),
        "pyenv" => Some(brew("pyenv")),
        "asdf" => Some(brew("asdf")),
        "volta" => Some(ToolActions {
            install: strs(&["brew", "install", "--cask", "volta"]),
            uninstall: strs(&["brew", "uninstall", "--cask", "volta"]),
            upgrade: strs(&["brew", "upgrade", "--cask", "volta"]),
        }),
        // Tools we can't safely manage (pip is bundled with python, etc.).
        _ => None,
    };
    def.actions = actions;
    def
}

/// Look up a single builtin tool by name.
pub fn find(name: &str) -> Option<ToolDefinition> {
    builtin_tools().into_iter().find(|t| t.name == name)
}
