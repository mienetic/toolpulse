// Shared types — mirror `src-tauri/src/tools/types.rs`.
// Keep these in sync when changing the Rust structs.

export type ToolCategory =
  | "javascript"
  | "python"
  | "rust"
  | "systems"
  | "runtime"
  | "infra"
  | "package_manager"
  | "version_manager";

export type InstallationSource =
  | "system"
  | "homebrew"
  | "nvm"
  | "pyenv"
  | "asdf"
  | "volta"
  | "conda"
  | "rustup"
  | "unknown";

export interface ToolInstallation {
  path: string;
  version?: string;
  source: InstallationSource;
  is_active: boolean;
}

export interface ToolStatus {
  name: string;
  display_name: string;
  category: ToolCategory;
  icon: string;
  color: string;
  installed_version?: string;
  latest_version?: string;
  is_outdated: boolean;
  path?: string;
  installations?: ToolInstallation[];
  checked_at: number;
  error?: string;
}

export interface InstalledPackage {
  name: string;
  version?: string;
  manager: string;
  /** On-disk size in bytes, when measurable. */
  size_bytes?: number;
}

export interface Snapshot {
  id: number;
  tool_name: string;
  version?: string;
  latest?: string;
  is_outdated: boolean;
  checked_at: number;
}

export interface DashboardSummary {
  total: number;
  installed: number;
  outdated: number;
  missing: number;
}

export type NotifyMode = "interval" | "daily_count" | "both" | "off";

export interface NotificationSettings {
  mode: NotifyMode;
  interval_hours: number;
  max_per_day: number;
  only_updates: boolean;
  dedupe_same_day: boolean;
  quiet_hours_start: number;
  quiet_hours_end: number;
}

export interface Settings {
  enabled_tools: string[];
  auto_check_on_start: boolean;
  auto_check_interval_hours: number;
  theme: "dark" | "light";
  overlay_enabled: boolean;
  notifications: NotificationSettings;
  /** Tool name -> pinned binary path. */
  selected_paths: Record<string, string>;
}

export type StatusKind = "updated" | "outdated" | "missing" | "unknown";

export function statusKind(t: ToolStatus): StatusKind {
  if (t.error && !t.installed_version) return "missing";
  if (t.installed_version && t.latest_version && t.is_outdated) return "outdated";
  if (t.installed_version && !t.latest_version) return "unknown";
  return "updated";
}

/** Human-readable label for an installation source. */
export function sourceLabel(s: InstallationSource): string {
  return {
    system: "System",
    homebrew: "Homebrew",
    nvm: "nvm",
    pyenv: "pyenv",
    asdf: "asdf",
    volta: "Volta",
    conda: "Conda",
    rustup: "rustup",
    unknown: "Unknown",
  }[s];
}

// --- Tool / package management ---------------------------------------------

export type ActionKind = "install" | "uninstall" | "upgrade";

export type Stream = "stdout" | "stderr";

export interface TerminalOutputLine {
  kind: "output";
  text: string;
  stream: Stream;
}
export interface TerminalStatusLine {
  kind: "status";
  text: string;
}
export interface TerminalDoneLine {
  kind: "done";
  success: boolean;
  message: string;
}
export type TerminalLine =
  | TerminalOutputLine
  | TerminalStatusLine
  | TerminalDoneLine;

/** Event payload from the backend's `toolpulse://terminal` event. */
export interface TerminalEvent {
  tag: string;
  line: TerminalLine;
}

/** A category label for the category filter dropdown. */
export const CATEGORY_LABELS: Record<ToolCategory, string> = {
  javascript: "JavaScript / TypeScript",
  python: "Python",
  rust: "Rust",
  systems: "Systems",
  runtime: "Runtime",
  infra: "Infra",
  package_manager: "Package Manager",
  version_manager: "Version Manager",
};

/** Status filter options. */
export type StatusFilter =
  | "all"
  | "updated"
  | "outdated"
  | "missing"
  | "multi_version";

/** Sort options. */
export type SortKey = "name" | "status" | "category";

/** Format bytes into a human-readable string (e.g. 1536 -> "1.5 MB"). */
export function formatBytes(bytes?: number): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

/** Sum the sizes of a package list. */
export function totalSize(packages: InstalledPackage[]): number {
  return packages.reduce((sum, p) => sum + (p.size_bytes ?? 0), 0);
}

// --- Project scanning -------------------------------------------------------

export type ProjectEcosystem =
  | "node"
  | "python"
  | "rust"
  | "go"
  | "ruby"
  | "php"
  | "java"
  | "dotnet";

export const ECOSYSTEM_META: Record<
  ProjectEcosystem,
  { label: string; icon: string; color: string }
> = {
  node: { label: "Node.js", icon: "node", color: "#5fa04e" },
  python: { label: "Python", icon: "python", color: "#3776ab" },
  rust: { label: "Rust", icon: "rust", color: "#dea584" },
  go: { label: "Go", icon: "go", color: "#00add8" },
  ruby: { label: "Ruby", icon: "ruby", color: "#cc342d" },
  php: { label: "PHP", icon: "php", color: "#777bb4" },
  java: { label: "Java", icon: "java", color: "#ed8b00" },
  dotnet: { label: ".NET", icon: "dotnet", color: "#512bd4" },
};

export interface ProjectDependency {
  name: string;
  version?: string;
  is_outdated?: boolean;
  latest?: string;
}

export interface DiscoveredProject {
  path: string;
  name: string;
  ecosystem: ProjectEcosystem;
  dependency_count: number;
  outdated_count?: number;
  size_bytes: number;
  manifest: string;
  /** `true` when the dir looks like a real project (hidden by default otherwise). */
  is_real_project: boolean;
}

/** An editor/IDE detected on the machine that can open a project. */
export interface DetectedIde {
  id: string;
  name: string;
  command: string;
  is_app: boolean;
}

/** A programming language whose standalone source files we can scan for. */
export type SourceLanguage =
  | "javascript"
  | "typescript"
  | "python"
  | "rust"
  | "go"
  | "ruby"
  | "php"
  | "java"
  | "swift"
  | "c"
  | "cpp";

export const LANGUAGE_META: Record<SourceLanguage, { label: string; color: string }> = {
  javascript: { label: "JavaScript", color: "#f7df1e" },
  typescript: { label: "TypeScript", color: "#3178c6" },
  python: { label: "Python", color: "#3776ab" },
  rust: { label: "Rust", color: "#dea584" },
  go: { label: "Go", color: "#00add8" },
  ruby: { label: "Ruby", color: "#cc342d" },
  php: { label: "PHP", color: "#777bb4" },
  java: { label: "Java", color: "#ed8b00" },
  swift: { label: "Swift", color: "#f05138" },
  c: { label: "C", color: "#a8b9cc" },
  cpp: { label: "C++", color: "#00599c" },
};

export const ALL_LANGUAGES: SourceLanguage[] = [
  "javascript", "typescript", "python", "rust", "go",
  "ruby", "php", "java", "swift", "c", "cpp",
];

/** A standalone source file found outside any project. */
export interface SourceFile {
  path: string;
  name: string;
  language: SourceLanguage;
  size_bytes: number;
}

/** Update availability from the GitHub releases endpoint. */
export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
}

/** System info collected for bug reports. */
export interface SystemInfo {
  app_version: string;
  os: string;
  arch: string;
  home: string;
}
