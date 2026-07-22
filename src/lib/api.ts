// Thin wrappers around `invoke`, so the rest of the frontend doesn't import
// the Tauri API directly.
//
// When running outside Tauri (e.g. `vite` in a plain browser, or snapshot
// tests), `invoke` rejects. We detect that and fall back to a built-in mock
// so the UI is still useful for design work and debugging.

import { invoke } from "@tauri-apps/api/core";
import type {
  ActionKind,
  DashboardSummary,
  DiscoveredProject,
  InstalledPackage,
  ProjectDependency,
  ProjectEcosystem,
  Settings,
  Snapshot,
  SourceLanguage,
  ToolStatus,
} from "../types";

const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    return invoke<T>(cmd, args);
  }
  return mock<T>(cmd, args);
}

export const api = {
  checkAllTools: () => call<ToolStatus[]>("check_all_tools"),
  checkTool: (name: string) => call<ToolStatus>("check_tool", { name }),
  getLatestVersion: (name: string) =>
    call<string | null>("get_latest_version", { name }),
  listInstalledPackages: (name: string) =>
    call<InstalledPackage[]>("list_installed_packages", { name }),
  getHistory: (days: number) => call<Snapshot[]>("get_history", { days }),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    call<void>("save_settings", { settings }),
  dashboardSummary: (statuses: ToolStatus[]) =>
    call<DashboardSummary>("dashboard_summary", { statuses }),
  notifyOutdated: (statuses: ToolStatus[]) =>
    call<number>("notify_outdated", { statuses }),
  settingsPath: () => call<string>("settings_path"),
  // --- Tool / package management ---
  manageTool: (name: string, action: ActionKind) =>
    call<void>("manage_tool", { name, action }),
  managePackage: (tool: string, pkg: string, action: ActionKind) =>
    call<void>("manage_package", { tool, package: pkg, action }),
  cancelRun: () => call<void>("cancel_run"),
  // --- Project scanning ---
  scanProjects: (root?: string) =>
    call<DiscoveredProject[]>("scan_projects", { root: root ?? null }),
  scanMachine: () => call<DiscoveredProject[]>("scan_machine"),
  pickFolder: () => call<string | null>("pick_folder"),
  scanProjectDeps: (manifest: string, ecosystem: ProjectEcosystem) =>
    call<ProjectDependency[]>("scan_project_deps", { manifest, ecosystem }),
  // --- Folder / IDE actions ---
  detectIdes: () => call<import("../types").DetectedIde[]>("detect_ides"),
  openFolder: (path: string) => call<void>("open_folder", { path }),
  openInIde: (path: string, ideCommand: string, isApp: boolean) =>
    call<void>("open_in_ide", { path, ideCommand, isApp }),
  trashFolder: (path: string) => call<void>("trash_folder", { path }),
  openTerminal: (path: string) => call<void>("open_terminal", { path }),
  // --- Source-file scanning ---
  scanSourceFiles: (languages: SourceLanguage[]) =>
    call<import("../types").SourceFile[]>("scan_source_files", { languages }),
  // --- Updater + system info ---
  checkForUpdate: () =>
    call<import("../types").UpdateInfo>("check_for_update"),
  installUpdate: () => call<void>("install_update"),
  systemInfo: () => call<import("../types").SystemInfo>("system_info"),
  get isTauri() {
    return isTauri;
  },
};

// --- Mock backend -----------------------------------------------------------
// Used only when the Tauri runtime is absent. Returns canned data shaped like
// the real backend so the UI can be developed in a browser.

const MOCK_TOOLS: ToolStatus[] = [
  {
    name: "node",
    display_name: "Node.js",
    category: "javascript",
    icon: "🟢",
    color: "#5fa04e",
    installed_version: "20.19.0",
    latest_version: "22.5.1",
    is_outdated: true,
    path: "/usr/local/bin/node",
    installations: [
      {
        path: "/usr/local/bin/node",
        version: "20.19.0",
        source: "homebrew",
        is_active: true,
      },
      {
        path: "/Users/apple/.nvm/versions/node/v18.20.0/bin/node",
        version: "18.20.0",
        source: "nvm",
        is_active: false,
      },
      {
        path: "/usr/bin/node",
        version: "16.14.0",
        source: "system",
        is_active: false,
      },
    ],
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "npm",
    display_name: "npm",
    category: "javascript",
    icon: "📦",
    color: "#cb3837",
    installed_version: "11.7.0",
    latest_version: "11.7.0",
    is_outdated: false,
    path: "/usr/local/bin/npm",
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "rust",
    display_name: "Rust",
    category: "rust",
    icon: "🦀",
    color: "#dea584",
    installed_version: "1.88.0",
    latest_version: "1.97.1",
    is_outdated: true,
    path: "/Users/apple/.cargo/bin/rustc",
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "zig",
    display_name: "Zig",
    category: "systems",
    icon: "⚡",
    color: "#f7a41d",
    installed_version: "0.15.2",
    latest_version: "0.15.2",
    is_outdated: false,
    path: "/usr/local/bin/zig",
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "docker",
    display_name: "Docker",
    category: "infra",
    icon: "🐳",
    color: "#2496ed",
    installed_version: "28.0.1",
    latest_version: "28.1.0",
    is_outdated: true,
    path: "/usr/local/bin/docker",
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "dotnet",
    display_name: ".NET SDK",
    category: "runtime",
    icon: "🌐",
    color: "#512bd4",
    installed_version: "7.0.101",
    latest_version: "9.0.100",
    is_outdated: true,
    path: "/usr/local/share/dotnet/dotnet",
    checked_at: Math.floor(Date.now() / 1000),
  },
  {
    name: "java",
    display_name: "Java",
    category: "runtime",
    icon: "☕",
    color: "#ed8b00",
    installed_version: "19.0.1",
    latest_version: "23",
    is_outdated: true,
    path: "/usr/bin/java",
    checked_at: Math.floor(Date.now() / 1000),
  },
];

const MOCK_SETTINGS: Settings = {
  enabled_tools: MOCK_TOOLS.map((t) => t.name),
  auto_check_on_start: true,
  auto_check_interval_hours: 24,
  theme: "dark",
  overlay_enabled: true,
  notifications: {
    mode: "both",
    interval_hours: 6,
    max_per_day: 4,
    only_updates: true,
    dedupe_same_day: true,
    quiet_hours_start: 22,
    quiet_hours_end: 8,
  },
  selected_paths: {},
};

async function mock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 300));
  switch (cmd) {
    case "check_all_tools":
      return MOCK_TOOLS as unknown as T;
    case "check_tool": {
      const found = MOCK_TOOLS.find((t) => t.name === (args?.name as string));
      return (found ?? MOCK_TOOLS[0]) as unknown as T;
    }
    case "get_latest_version":
      return "22.5.1" as unknown as T;
    case "list_installed_packages":
      return [
        { name: "typescript", version: "5.8.3", manager: "npm", size_bytes: 68_000_000 },
        { name: "vite", version: "7.0.4", manager: "npm", size_bytes: 12_500_000 },
        { name: "eslint", version: "9.0.0", manager: "npm", size_bytes: 23_800_000 },
      ] as unknown as T;
    case "get_history":
      return [] as unknown as T;
    case "get_settings":
      return MOCK_SETTINGS as unknown as T;
    case "save_settings":
      return undefined as unknown as T;
    case "dashboard_summary": {
      const s = (args?.statuses as ToolStatus[]) ?? [];
      return {
        total: s.length,
        installed: s.filter((t) => t.installed_version).length,
        outdated: s.filter((t) => t.is_outdated).length,
        missing: s.filter((t) => !t.installed_version).length,
      } as unknown as T;
    }
    case "notify_outdated":
      return 3 as unknown as T;
    case "settings_path":
      return "/mock/settings.json" as unknown as T;
    case "manage_tool":
    case "manage_package":
      // Mock: resolve successfully after a brief delay.
      return undefined as unknown as T;
    case "cancel_run":
      return undefined as unknown as T;
    case "scan_projects":
    case "scan_machine":
      return [
        {
          path: "/Users/apple/projects/web/my-app",
          name: "my-app",
          ecosystem: "node",
          dependency_count: 42,
          outdated_count: 3,
          size_bytes: 256_000_000,
          manifest: "/Users/apple/projects/web/my-app/package.json",
          is_real_project: true,
        },
        {
          path: "/Users/apple/projects/web/admin",
          name: "admin",
          ecosystem: "node",
          dependency_count: 28,
          outdated_count: 1,
          size_bytes: 180_000_000,
          manifest: "/Users/apple/projects/web/admin/package.json",
          is_real_project: true,
        },
        {
          path: "/Users/apple/projects/api",
          name: "api",
          ecosystem: "python",
          dependency_count: 18,
          size_bytes: 45_000_000,
          manifest: "/Users/apple/projects/api/requirements.txt",
          is_real_project: true,
        },
        {
          path: "/Users/apple/notes/scratch",
          name: "scratch",
          ecosystem: "python",
          dependency_count: 0,
          size_bytes: 1_200_000,
          manifest: "/Users/apple/notes/scratch/requirements.txt",
          is_real_project: false,
        },
        {
          path: "/Users/apple/work/cli-tool",
          name: "cli-tool",
          ecosystem: "rust",
          dependency_count: 12,
          size_bytes: 32_000_000,
          manifest: "/Users/apple/work/cli-tool/Cargo.toml",
          is_real_project: true,
        },
      ] as unknown as T;
    case "pick_folder":
      return "/Users/apple/projects" as unknown as T;
    case "scan_project_deps":
      return [
        { name: "react", version: "^18.0.0", is_outdated: true, latest: "19.1.0" },
        { name: "typescript", version: "^5.4.0", is_outdated: false, latest: "5.4.0" },
      ] as unknown as T;
    case "detect_ides":
      return [
        { id: "code", name: "VS Code", command: "code", is_app: false },
        { id: "cursor", name: "Cursor", command: "cursor", is_app: false },
      ] as unknown as T;
    case "open_folder":
    case "open_in_ide":
    case "trash_folder":
    case "open_terminal":
      return undefined as unknown as T;
    case "scan_source_files":
      return [
        { path: "/Users/apple/scripts/fetch_data.py", name: "fetch_data.py", language: "python", size_bytes: 2400 },
        { path: "/Users/apple/scripts/convert.ts", name: "convert.ts", language: "typescript", size_bytes: 1800 },
        { path: "/Users/apple/tmp/hello.rs", name: "hello.rs", language: "rust", size_bytes: 320 },
      ] as unknown as T;
    case "check_for_update":
      return { available: false, version: null, body: null } as unknown as T;
    case "install_update":
      return undefined as unknown as T;
    case "system_info":
      return {
        app_version: "0.1.0",
        os: "macos",
        arch: "aarch64",
        home: "/Users/demo",
      } as unknown as T;
    default:
      return undefined as unknown as T;
  }
}
