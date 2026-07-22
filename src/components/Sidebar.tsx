import {
  CheckCircle2,
  AlertTriangle,
  XCircle,
  HelpCircle,
  Layers,
} from "lucide-react";
import { ToolIcon } from "../lib/icons";
import { statusKind, type ToolStatus } from "../types";

interface SidebarProps {
  tools: ToolStatus[];
  selectedName: string | null;
  onSelect: (name: string) => void;
}

const STATUS_DOT = {
  updated: { color: "var(--green)", Icon: CheckCircle2 },
  outdated: { color: "var(--yellow)", Icon: AlertTriangle },
  missing: { color: "var(--red)", Icon: XCircle },
  unknown: { color: "var(--text-dim)", Icon: HelpCircle },
};

/**
 * Master list of tools. Each row shows the tool icon, name, version, and a
 * status indicator. Clicking selects the tool for the detail panel.
 */
export function Sidebar({ tools, selectedName, onSelect }: SidebarProps) {
  return (
    <nav className="sidebar" aria-label="Tool list">
      {tools.length === 0 ? (
        <div className="sidebar__empty muted">No tools match the filters.</div>
      ) : (
        <ul className="sidebar__list">
          {tools.map((t) => {
            const kind = statusKind(t);
            const dot = STATUS_DOT[kind];
            const isSel = t.name === selectedName;
            const multi = (t.installations?.length ?? 0) > 1;
            return (
              <li key={t.name}>
                <button
                  className="sidebar__item"
                  data-selected={isSel}
                  onClick={() => onSelect(t.name)}
                >
                  <span className="sidebar__icon" style={{ color: t.color }}>
                    <ToolIcon name={t.name} emoji={t.icon} size={18} />
                  </span>
                  <span className="sidebar__text">
                    <span className="sidebar__name">{t.display_name}</span>
                    <span className="sidebar__version">
                      {t.installed_version ?? "not installed"}
                    </span>
                  </span>
                  {multi && (
                    <span className="sidebar__multi" title="Multiple versions">
                      <Layers size={11} />
                    </span>
                  )}
                  <dot.Icon
                    size={13}
                    style={{ color: dot.color, flexShrink: 0 }}
                  />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </nav>
  );
}
