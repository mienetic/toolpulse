import { useCallback, useEffect, useMemo, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import { ChevronDown, ChevronUp, Terminal as TerminalIcon } from "lucide-react";
import { Sidebar } from "./components/Sidebar";
import { DetailPanel } from "./components/DetailPanel";
import { Dashboard } from "./components/Dashboard";
import { Toolbar } from "./components/Toolbar";
import { HistoryView } from "./components/HistoryView";
import { ProjectsView } from "./components/ProjectsView";
import { FilesView } from "./components/FilesView";
import { FiltersBar, applyFilters, type ToolFilters } from "./components/FiltersBar";
import { TerminalPanel } from "./components/TerminalPanel";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { AboutDialog } from "./components/AboutDialog";
import { useTools } from "./hooks/useTools";
import { useTerminalRun } from "./hooks/useTerminalRun";
import { useProjectScan } from "./hooks/useProjectScan";
import { useFileScan } from "./hooks/useFileScan";
import { api } from "./lib/api";
import type { Settings, ToolCategory } from "./types";
import "./App.css";
import "./styles/theme.css";

type Tab = "tools" | "projects" | "files" | "history";

const DEFAULT_FILTERS: ToolFilters = {
  query: "",
  category: "all",
  status: "all",
  sort: "name",
};

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [tab, setTab] = useState<Tab>("tools");
  const [notificationsOn, setNotificationsOn] = useState(true);
  const [filters, setFilters] = useState<ToolFilters>(DEFAULT_FILTERS);

  // Scan state lives here (not in the view components) so it persists across
  // tab switches — switching away and back keeps the last scan intact.
  const projectScan = useProjectScan();
  const fileScan = useFileScan();

  // Auto-updater: check once on mount (silent), surface a badge when available.
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [aboutOpen, setAboutOpen] = useState(false);
  useEffect(() => {
    if (!api.isTauri) return;
    api
      .checkForUpdate()
      .then((info) => {
        if (info.available && info.version) setUpdateVersion(info.version);
      })
      .catch(() => {});
  }, []);

  const installUpdate = useCallback(async () => {
    try {
      await api.installUpdate();
    } catch (e) {
      console.error("update failed:", e);
    }
  }, []);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [terminalCollapsed, setTerminalCollapsed] = useState(false);

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => {});
  }, []);

  const autoCheck = settings?.auto_check_on_start ?? true;
  const { tools, loading, error, lastChecked, summary, refresh, refreshOne } =
    useTools(autoCheck);

  const run = useTerminalRun();

  const theme = settings?.theme ?? "dark";
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    if (summary) {
      emit("toolpulse://summary", summary).catch(() => {});
    }
  }, [summary]);

  useEffect(() => {
    if (notificationsOn && tools.length > 0) {
      api.notifyOutdated(tools).catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lastChecked]);

  const toggleTheme = useCallback(async () => {
    if (!settings) return;
    const next: Settings = {
      ...settings,
      theme: settings.theme === "dark" ? "light" : "dark",
    };
    setSettings(next);
    await api.saveSettings(next);
  }, [settings]);

  const enabledTools = useMemo(() => {
    if (!settings) return tools;
    return tools.filter((t) => settings.enabled_tools.includes(t.name));
  }, [tools, settings]);

  const presentCategories = useMemo(() => {
    const set = new Set<ToolCategory>();
    enabledTools.forEach((t) => set.add(t.category));
    return [...set];
  }, [enabledTools]);

  const visibleTools = useMemo(
    () => applyFilters(enabledTools, filters),
    [enabledTools, filters],
  );

  // Keep the selection valid when the filtered list changes.
  useEffect(() => {
    if (selectedName && !visibleTools.some((t) => t.name === selectedName)) {
      setSelectedName(visibleTools[0]?.name ?? null);
    } else if (!selectedName && visibleTools.length > 0) {
      setSelectedName(visibleTools[0].name);
    }
  }, [visibleTools, selectedName]);

  const selectedTool = useMemo(
    () => visibleTools.find((t) => t.name === selectedName) ?? null,
    [visibleTools, selectedName],
  );

  if (!settings) {
    return (
      <div className="app">
        <div className="muted">Loading…</div>
      </div>
    );
  }

  const hasTerminals = run.terminals.length > 0;

  return (
    <ErrorBoundary>
    <div className="app">
      <Toolbar
        loading={loading}
        onRefresh={refresh}
        onToggleTheme={toggleTheme}
        theme={theme}
        notificationsEnabled={notificationsOn}
        onToggleNotifications={() => setNotificationsOn((v) => !v)}
        updateAvailable={updateVersion !== null}
        onUpdate={installUpdate}
        onAbout={() => setAboutOpen(true)}
      />

      <Dashboard summary={summary} lastChecked={lastChecked} loading={loading} />

      <div className="tabs">
        <button
          className="tab"
          data-active={tab === "tools"}
          onClick={() => setTab("tools")}
        >
          Tools
        </button>
        <button
          className="tab"
          data-active={tab === "projects"}
          onClick={() => setTab("projects")}
        >
          Projects
        </button>
        <button
          className="tab"
          data-active={tab === "files"}
          onClick={() => setTab("files")}
        >
          Files
        </button>
        <button
          className="tab"
          data-active={tab === "history"}
          onClick={() => setTab("history")}
        >
          History
        </button>
      </div>

      <main className="content">
        {error && <div className="error-banner">{error}</div>}

        {tab === "tools" ? (
          <div className="workspace">
            <aside className="workspace__sidebar">
              <FiltersBar
                filters={filters}
                onChange={setFilters}
                categories={presentCategories}
              />
              <Sidebar
                tools={visibleTools}
                selectedName={selectedName}
                onSelect={setSelectedName}
              />
            </aside>
            <div className="workspace__detail">
              <DetailPanel
                tool={selectedTool}
                settings={settings}
                onRefresh={refreshOne}
                onSettingsChange={setSettings}
                onAction={(action) =>
                  selectedTool &&
                  run.requestTool(
                    selectedTool.name,
                    selectedTool.display_name,
                    action,
                    `tool:${selectedTool.name}`,
                    () => refreshOne(selectedTool.name),
                  )
                }
                onPackageAction={(pkg, action) =>
                  selectedTool &&
                  run.requestPackage(
                    selectedTool.name,
                    pkg,
                    action,
                    `pkg:${selectedTool.name}:${pkg}`,
                    () => {},
                  )
                }
              />
            </div>
          </div>
        ) : tab === "projects" ? (
          <ProjectsView
            projects={projectScan.projects}
            scanning={projectScan.scanning}
            scanRoot={projectScan.scanRoot}
            onScanMachine={projectScan.scanMachine}
            onPickFolder={projectScan.pickFolder}
            onRemoveProject={projectScan.removeProject}
          />
        ) : tab === "files" ? (
          <FilesView
            files={fileScan.files}
            scanning={fileScan.scanning}
            onScan={fileScan.scan}
          />
        ) : (
          <HistoryView />
        )}
      </main>

      {/* Persistent terminal panel docked at the bottom. */}
      {hasTerminals && (
        <div
          className="terminal-dock"
          data-collapsed={terminalCollapsed}
        >
          <button
            className="terminal-dock__toggle"
            onClick={() => setTerminalCollapsed((v) => !v)}
          >
            <TerminalIcon size={13} />
            <span>Terminal ({run.terminals.length})</span>
            {terminalCollapsed ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
          {!terminalCollapsed && (
            <div className="terminal-dock__panels">
              {run.terminals.map((t) => (
                <TerminalPanel
                  key={t.tag}
                  tag={t.tag}
                  title={t.title}
                  onClose={() => run.closeTerminal(t.tag)}
                />
              ))}
            </div>
          )}
        </div>
      )}

      <ConfirmDialog
        open={run.pending !== null}
        action={run.pending?.action ?? "install"}
        subject={run.pending?.subject ?? ""}
        detail={run.pending?.detail}
        onConfirm={run.confirm}
        onCancel={run.cancel}
      />
      <AboutDialog
        open={aboutOpen}
        onClose={() => setAboutOpen(false)}
        appVersion="0.1.0"
        updateAvailable={updateVersion !== null}
        updateVersion={updateVersion}
        onUpdate={installUpdate}
      />
    </div>
    </ErrorBoundary>
  );
}

export default App;
