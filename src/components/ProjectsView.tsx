import { useCallback, useEffect, useMemo, useState } from "react";
import {
  FolderSearch,
  RefreshCw,
  Search,
  Cpu,
  FolderOpen,
  Terminal,
  FolderTree,
  Copy,
  Trash2,
  Code2,
} from "lucide-react";
import { api } from "../lib/api";
import { buildTree } from "../lib/projectTree";
import { ProjectTree, type ContextAction } from "./ProjectTree";
import { ContextMenu, type ContextMenuItem } from "./ContextMenu";
import { ConfirmDialog } from "./ConfirmDialog";
import {
  ECOSYSTEM_META,
  formatBytes,
  type DetectedIde,
  type DiscoveredProject,
  type ProjectDependency,
  type ProjectEcosystem,
} from "../types";

interface ProjectsViewProps {
  projects: DiscoveredProject[];
  scanning: boolean;
  scanRoot: string | null;
  onScanMachine: () => void;
  onPickFolder: () => void;
  onRemoveProject: (path: string) => void;
}

/**
 * Whole-machine project scanner view.
 *
 * The scan state (projects, scanning, scanRoot) is owned by the parent
 * (`useProjectScan`) so it survives tab switches — this component only holds
 * ephemeral UI state (filters, expanded rows, context menu).
 */
export function ProjectsView({
  projects,
  scanning,
  scanRoot,
  onScanMachine,
  onPickFolder,
  onRemoveProject,
}: ProjectsViewProps) {
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [deps, setDeps] = useState<Record<string, ProjectDependency[]>>({});
  const [loadingDeps, setLoadingDeps] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [ecoFilter, setEcoFilter] = useState<ProjectEcosystem | "all">("all");

  // Toggle a project's deps panel open/closed, loading deps on first open.
  async function toggleProject(p: DiscoveredProject) {
    setExpanded((prev) => ({ ...prev, [p.path]: !prev[p.path] }));
    if (!deps[p.path]) {
      setLoadingDeps(p.path);
      try {
        const d = await api.scanProjectDeps(p.manifest, p.ecosystem);
        setDeps((prev) => ({ ...prev, [p.path]: d }));
      } catch {
        setDeps((prev) => ({ ...prev, [p.path]: [] }));
      } finally {
        setLoadingDeps(null);
      }
    }
  }
  const [ides, setIdes] = useState<DetectedIde[]>([]);
  const [trashTarget, setTrashTarget] = useState<DiscoveredProject | null>(null);
  const [showNonProjects, setShowNonProjects] = useState(false);
  // Context menu state: which row was right-clicked and where.
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    path: string;
    project: DiscoveredProject | null;
  } | null>(null);

  // Detect installed editors/IDEs once on mount.
  useEffect(() => {
    api
      .detectIdes()
      .then((i) => setIdes(Array.isArray(i) ? i : []))
      .catch(() => {});
  }, []);

  // Resolve the dependency-install directory for a project's ecosystem.
  // Returns null when there's no canonical deps folder to open.
  const depsFolderFor = useCallback(
    (project: DiscoveredProject): string | null => {
      const base = project.path;
      const candidates: Record<ProjectEcosystem, string[]> = {
        node: ["node_modules"],
        rust: ["target"],
        python: [".venv", "venv", "__pypackages__"],
        go: ["vendor"],
        ruby: ["vendor/bundle", ".bundle"],
        php: ["vendor"],
        java: ["target", "build"],
        dotnet: ["bin", "obj"],
      };
      for (const c of candidates[project.ecosystem] ?? []) {
        return `${base}/${c}`;
      }
      return null;
    },
    [],
  );

  const handleContextAction = useCallback(
    (
      action: ContextAction,
      path: string,
      project: DiscoveredProject | null,
    ) => {
      // IDE selection carries the chosen editor inline.
      if (typeof action === "object" && "ide" in action) {
        api
          .openInIde(path, action.ide.command, action.ide.is_app)
          .catch((e) => setError(String(e)));
        return;
      }
      switch (action) {
        case "open-folder":
          api.openFolder(path).catch((e) => setError(String(e)));
          break;
        case "open-terminal":
          api.openTerminal(path).catch((e) => setError(String(e)));
          break;
        case "open-deps":
          if (project) {
            const depsPath = depsFolderFor(project);
            if (depsPath) api.openFolder(depsPath).catch((e) => setError(String(e)));
          }
          break;
        case "copy-path":
          navigator.clipboard?.writeText(path).catch(() => {});
          break;
        case "copy-name":
          navigator.clipboard?.writeText(project?.name ?? path).catch(() => {});
          break;
        case "trash":
          if (project) setTrashTarget(project);
          break;
      }
    },
    [depsFolderFor],
  );

  // Build the context menu items for a given row target.
  const confirmTrash = useCallback(async () => {
    if (!trashTarget) return;
    const path = trashTarget.path;
    setTrashTarget(null);
    try {
      await api.trashFolder(path);
      onRemoveProject(path);
    } catch (e) {
      setError(String(e));
    }
  }, [trashTarget, onRemoveProject]);

  const buildMenuItems = useCallback(
    (path: string, project: DiscoveredProject | null): ContextMenuItem[] => {
      const items: ContextMenuItem[] = [
        {
          id: "open-folder",
          label: "Open folder",
          icon: <FolderOpen size={13} />,
          onSelect: () => handleContextAction("open-folder", path, project),
        },
      ];
      // Each detected IDE becomes its own menu entry.
      for (const ide of ides) {
        items.push({
          id: `ide-${ide.id}`,
          label: `Open in ${ide.name}`,
          icon: <Code2 size={13} />,
          onSelect: () =>
            handleContextAction({ ide }, path, project),
        });
      }
      items.push(
        {
          id: "open-terminal",
          label: "Open terminal here",
          icon: <Terminal size={13} />,
          onSelect: () => handleContextAction("open-terminal", path, project),
        },
        { id: "sep1", label: "", separator: true },
        {
          id: "open-deps",
          label: "Open dependencies folder",
          icon: <FolderTree size={13} />,
          disabled: !project || !depsFolderFor(project),
          onSelect: () => handleContextAction("open-deps", path, project),
        },
        {
          id: "copy-path",
          label: "Copy path",
          icon: <Copy size={13} />,
          onSelect: () => handleContextAction("copy-path", path, project),
        },
        {
          id: "copy-name",
          label: "Copy name",
          icon: <Copy size={13} />,
          onSelect: () => handleContextAction("copy-name", path, project),
        },
      );
      if (project) {
        items.push(
          { id: "sep2", label: "", separator: true },
          {
            id: "trash",
            label: "Move to Trash",
            icon: <Trash2 size={13} />,
            danger: true,
            onSelect: () => handleContextAction("trash", path, project),
          },
        );
      }
      return items;
    },
    [ides, depsFolderFor, handleContextAction],
  );
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return projects
      .filter((p) => {
        if (!showNonProjects && !p.is_real_project) return false;
        if (ecoFilter !== "all" && p.ecosystem !== ecoFilter) return false;
        if (q && !`${p.name} ${p.path}`.toLowerCase().includes(q)) return false;
        return true;
      })
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [projects, query, ecoFilter, showNonProjects]);

  // Build a folder tree from the filtered projects so they're grouped by
  // their directory structure rather than shown as a flat list.
  const tree = useMemo(() => buildTree(filtered), [filtered]);

  const presentEcos = useMemo(() => {
    const set = new Set<ProjectEcosystem>();
    projects.forEach((p) => set.add(p.ecosystem));
    return [...set];
  }, [projects]);

  const totalSize = useMemo(
    () => filtered.reduce((s, p) => s + p.size_bytes, 0),
    [filtered],
  );

  return (
    <div className="projects">
      <div className="projects__header">
        <div className="projects__title">
          <FolderSearch size={16} />
          <span>Project Scanner</span>
        </div>
        <div className="projects__actions">
          <button
            className="btn-secondary"
            onClick={onPickFolder}
            disabled={scanning}
          >
            <FolderSearch size={14} /> Choose folder…
          </button>
          <button
            className="btn-primary"
            onClick={onScanMachine}
            disabled={scanning}
          >
            {scanning ? (
              <RefreshCw size={14} className="spin" />
            ) : (
              <Cpu size={14} />
            )}
            <span>{scanning ? "Scanning…" : "Scan Machine"}</span>
          </button>
        </div>
      </div>

      {scanRoot && (
        <div className="projects__root muted">
          {scanning ? (
            <>
              <RefreshCw size={11} className="spin" /> Scanning{" "}
              <code>{scanRoot}</code>… found {projects.length} so far
            </>
          ) : (
            <>
              Scanned: <code>{scanRoot}</code> · {projects.length} projects ·{" "}
              {formatBytes(totalSize)}
            </>
          )}
        </div>
      )}

      {error && <div className="error-banner">{error}</div>}

      {projects.length > 0 && (
        <div className="projects__filters">
          <div className="filters__search">
            <Search size={14} />
            <input
              type="text"
              placeholder="Search projects…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="projects__eco-chips">
            <button
              className="eco-chip"
              data-active={ecoFilter === "all"}
              onClick={() => setEcoFilter("all")}
            >
              All
            </button>
            {presentEcos.map((e) => (
              <button
                key={e}
                className="eco-chip"
                data-active={ecoFilter === e}
                style={{
                  borderColor: ecoFilter === e ? ECOSYSTEM_META[e].color : undefined,
                }}
                onClick={() => setEcoFilter(e)}
              >
                {ECOSYSTEM_META[e].label}
              </button>
            ))}
            <label className="projects__toggle" title="Show loose manifests that aren't full projects">
              <input
                type="checkbox"
                checked={showNonProjects}
                onChange={(e) => setShowNonProjects(e.target.checked)}
              />
              <span>Show non-projects</span>
            </label>
          </div>
        </div>
      )}

      {scanning ? (
        <div className="scan-progress">
          <RefreshCw size={20} className="spin" />
          <div className="scan-progress__text">
            <div className="scan-progress__title">Scanning machine…</div>
            <div className="muted">
              {projects.length > 0
                ? `Found ${projects.length} projects so far`
                : "Walking directories…"}
            </div>
          </div>
        </div>
      ) : projects.length === 0 ? (
        <div className="empty">
          No projects scanned yet. Click "Scan Machine" or choose a folder.
        </div>
      ) : (
        <div
          onContextMenu={(e) => {
            // Capture the right-clicked row so we can build its menu.
            e.preventDefault();
            const row = (e.target as HTMLElement).closest(
              ".tree__folder-head, .tree__project-head",
            );
            let path = "";
            let project: DiscoveredProject | null = null;
            if (row) {
              // Leaf rows carry their path in the sibling `.tree__project-path`.
              const leaf = row.parentElement;
              const pathEl = leaf?.querySelector(".tree__project-path");
              path =
                pathEl?.textContent?.trim() ??
                row.querySelector(".tree__folder-name")?.textContent?.trim() ??
                "";
              // Match project by path from the scanned list.
              project = projects.find((p) => p.path === path) ?? null;
              if (!project && path) {
                // Folder row: path is the breadcrumb text; use it as-is.
              }
            }
            setMenu({ x: e.clientX, y: e.clientY, path: path || "$HOME", project });
          }}
        >
          <ProjectTree
            tree={tree}
            expandedProjects={expanded}
            deps={deps}
            loadingDeps={loadingDeps}
            ides={ides}
            onToggleProject={toggleProject}
            onContextAction={(action, p, proj) =>
              handleContextAction(action, p, proj)
            }
          />
        </div>
      )}

      {/* Right-click context menu. */}
      <ContextMenu
        open={menu !== null}
        x={menu?.x ?? 0}
        y={menu?.y ?? 0}
        items={menu ? buildMenuItems(menu.path, menu.project) : []}
        onClose={() => setMenu(null)}
      />

      {/* Trash confirmation — destructive, so always confirm. */}
      <ConfirmDialog
        open={trashTarget !== null}
        action="uninstall"
        subject={trashTarget?.name ?? ""}
        detail={
          trashTarget
            ? `${trashTarget.path}\n${formatBytes(trashTarget.size_bytes)}`
            : undefined
        }
        onConfirm={confirmTrash}
        onCancel={() => setTrashTarget(null)}
      />
    </div>
  );
}
