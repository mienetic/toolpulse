import { useEffect, useMemo, useState } from "react";
import {
  Download,
  Trash2,
  ArrowUpCircle,
  RefreshCw,
  Search,
  Layers,
  HardDrive,
} from "lucide-react";
import { api } from "../lib/api";
import { ToolIcon } from "../lib/icons";
import {
  formatBytes,
  sourceLabel,
  statusKind,
  totalSize,
  type ActionKind,
  type InstalledPackage,
  type Settings,
  type ToolStatus,
} from "../types";

interface DetailPanelProps {
  tool: ToolStatus | null;
  settings: Settings | null;
  onRefresh: (name: string) => void;
  onSettingsChange: (next: Settings) => void;
  onAction: (action: ActionKind) => void;
  onPackageAction: (pkg: string, action: ActionKind) => void;
}

const STATUS_LABEL = {
  updated: { text: "Up to date", color: "var(--green)" },
  outdated: { text: "Update available", color: "var(--yellow)" },
  missing: { text: "Not installed", color: "var(--red)" },
  unknown: { text: "Unknown", color: "var(--text-dim)" },
};

/**
 * Right-hand detail view for the selected tool.
 *
 * Shows versions, multi-version picker, action buttons, and the global package
 * list (with per-package and total on-disk sizes).
 */
export function DetailPanel({
  tool,
  settings,
  onRefresh,
  onSettingsChange,
  onAction,
  onPackageAction,
}: DetailPanelProps) {
  const [packages, setPackages] = useState<InstalledPackage[] | null>(null);
  const [loadingPkgs, setLoadingPkgs] = useState(false);
  const [pkgQuery, setPkgQuery] = useState("");

  // Load packages when the selected tool changes.
  useEffect(() => {
    setPackages(null);
    setPkgQuery("");
    if (!tool || !tool.installed_version) return;
    let alive = true;
    setLoadingPkgs(true);
    api
      .listInstalledPackages(tool.name)
      .then((pkgs) => {
        if (alive) setPackages(pkgs);
      })
      .catch(() => {
        if (alive) setPackages([]);
      })
      .finally(() => {
        if (alive) setLoadingPkgs(false);
      });
    return () => {
      alive = false;
    };
  }, [tool?.name, tool?.installed_version]);

  const filteredPackages = useMemo(() => {
    if (!packages) return [];
    const q = pkgQuery.trim().toLowerCase();
    if (!q) return packages;
    return packages.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        (p.version?.toLowerCase().includes(q) ?? false),
    );
  }, [packages, pkgQuery]);

  const sumBytes = useMemo(() => totalSize(packages ?? []), [packages]);

  if (!tool) {
    return (
      <div className="detail detail--empty">
        <div className="detail__placeholder">
          <Layers size={32} />
          <p>Select a tool to see its details.</p>
        </div>
      </div>
    );
  }

  const kind = statusKind(tool);
  const status = STATUS_LABEL[kind];
  const installs = tool.installations ?? [];
  const hasMultiple = installs.length > 1;
  const selectedPath = settings?.selected_paths?.[tool.name];

  function selectInstallation(path: string) {
    if (!settings || !tool) return;
    const next: Settings = {
      ...settings,
      selected_paths: { ...settings.selected_paths, [tool.name]: path },
    };
    onSettingsChange(next);
    api.saveSettings(next).then(() => onRefresh(tool.name));
  }

  return (
    <section className="detail">
      {/* Header */}
      <header
        className="detail__header"
        style={{
          background: `linear-gradient(135deg, ${tool.color}22, transparent 80%)`,
        }}
      >
        <span className="detail__icon" style={{ color: tool.color }}>
          <ToolIcon name={tool.name} emoji={tool.icon} size={36} />
        </span>
        <div className="detail__titleblock">
          <h1 className="detail__name">{tool.display_name}</h1>
          <code className="detail__id">{tool.name}</code>
        </div>
        <button
          className="icon-btn"
          title="Re-check"
          onClick={() => onRefresh(tool.name)}
        >
          <RefreshCw size={15} />
        </button>
      </header>

      {/* Version + status grid */}
      <div className="detail__grid">
        <div className="detail__field">
          <span className="detail__field-label">Installed</span>
          <span className="detail__field-value">
            {tool.installed_version ?? "—"}
          </span>
        </div>
        <div className="detail__field">
          <span className="detail__field-label">Latest</span>
          <span className="detail__field-value detail__field-value--muted">
            {tool.latest_version ?? "—"}
          </span>
        </div>
        <div className="detail__field">
          <span className="detail__field-label">Status</span>
          <span className="detail__field-value" style={{ color: status.color }}>
            {status.text}
          </span>
        </div>
        {tool.path && (
          <div className="detail__field detail__field--wide">
            <span className="detail__field-label">Binary path</span>
            <code className="detail__path" title={tool.path}>
              {tool.path}
            </code>
          </div>
        )}
      </div>

      {/* Action buttons */}
      <div className="detail__actions">
        {!tool.installed_version && (
          <button
            className="action-btn action-btn--primary"
            onClick={() => onAction("install")}
          >
            <Download size={13} /> Install
          </button>
        )}
        {tool.is_outdated && (
          <button
            className="action-btn action-btn--accent"
            onClick={() => onAction("upgrade")}
          >
            <ArrowUpCircle size={13} /> Upgrade
          </button>
        )}
        {tool.installed_version && (
          <button
            className="action-btn action-btn--danger"
            onClick={() => onAction("uninstall")}
          >
            <Trash2 size={13} /> Uninstall
          </button>
        )}
      </div>

      {/* Multi-version picker */}
      {hasMultiple && (
        <div className="detail__installs">
          <div className="detail__section-title">
            <Layers size={13} />
            <span>
              {installs.length} versions detected — choose default
            </span>
          </div>
          {installs.map((inst) => {
            const isChosen =
              (selectedPath && inst.path === selectedPath) ||
              (!selectedPath && inst.is_active);
            return (
              <label
                key={inst.path}
                className="install-row"
                data-active={isChosen}
                title={inst.path}
              >
                <input
                  type="radio"
                  name={`install-${tool.name}`}
                  checked={isChosen}
                  onChange={() => selectInstallation(inst.path)}
                />
                <span className="install-row__version">
                  {inst.version ?? "unknown"}
                </span>
                <span className="install-row__source">
                  {sourceLabel(inst.source)}
                </span>
                <span className="install-row__path">{inst.path}</span>
              </label>
            );
          })}
        </div>
      )}

      {/* Package list with sizes */}
      {tool.installed_version && (
        <div className="detail__packages">
          <div className="detail__section-title">
            <HardDrive size={13} />
            <span>Packages</span>
            {packages && packages.length > 0 && (
              <span className="detail__total">
                {packages.length} · {formatBytes(sumBytes)} total
              </span>
            )}
          </div>

          {loadingPkgs ? (
            <div className="muted">Loading packages…</div>
          ) : packages && packages.length > 0 ? (
            <>
              {packages.length > 4 && (
                <div className="pkg-search">
                  <Search size={12} />
                  <input
                    type="text"
                    placeholder="Filter packages…"
                    value={pkgQuery}
                    onChange={(e) => setPkgQuery(e.target.value)}
                  />
                </div>
              )}
              <ul className="pkg-table">
                <li className="pkg-table__row pkg-table__row--head">
                  <span>Package</span>
                  <span>Version</span>
                  <span>Manager</span>
                  <span className="pkg-table__size">Size</span>
                  <span></span>
                </li>
                {filteredPackages.map((p, i) => (
                  <li key={`${p.name}-${i}`} className="pkg-table__row">
                    <span className="pkg-table__name">{p.name}</span>
                    <span className="pkg-table__version">
                      {p.version ?? "—"}
                    </span>
                    <span className="pkg-table__manager">{p.manager}</span>
                    <span className="pkg-table__size">
                      {formatBytes(p.size_bytes)}
                    </span>
                    <span className="pkg-table__row-actions">
                      <button
                        className="icon-btn pkg-action"
                        title={`Upgrade ${p.name}`}
                        onClick={() => onPackageAction(p.name, "upgrade")}
                      >
                        <ArrowUpCircle size={13} />
                      </button>
                      <button
                        className="icon-btn pkg-action pkg-action--danger"
                        title={`Uninstall ${p.name}`}
                        onClick={() => onPackageAction(p.name, "uninstall")}
                      >
                        <Trash2 size={13} />
                      </button>
                    </span>
                  </li>
                ))}
                {filteredPackages.length === 0 && (
                  <li className="pkg-table__row">
                    <span className="muted">
                      No packages match "{pkgQuery}".
                    </span>
                  </li>
                )}
              </ul>
            </>
          ) : (
            <div className="muted">No packages found for this tool.</div>
          )}
        </div>
      )}
    </section>
  );
}
