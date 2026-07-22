import { useEffect, useState } from "react";
import { History } from "lucide-react";
import { api } from "../lib/api";
import type { Snapshot } from "../types";

const HISTORY_DAYS = 30;

export function HistoryView() {
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    api
      .getHistory(HISTORY_DAYS)
      .then((rows) => {
        if (alive) setSnapshots(rows);
      })
      .catch(() => {})
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  // Group by tool name for a per-tool timeline.
  const byTool = new Map<string, Snapshot[]>();
  for (const s of snapshots) {
    const list = byTool.get(s.tool_name) ?? [];
    list.push(s);
    byTool.set(s.tool_name, list);
  }
  const toolNames = [...byTool.keys()].sort();

  return (
    <div className="history">
      <div className="history__title">
        <History size={16} />
        <span>Version history (last {HISTORY_DAYS} days)</span>
      </div>
      {loading ? (
        <div className="muted">Loading…</div>
      ) : snapshots.length === 0 ? (
        <div className="muted">No history yet. Run a scan to start tracking.</div>
      ) : (
        <div className="history__grid">
          {toolNames.map((name) => (
            <div key={name} className="history__tool">
              <div className="history__tool-name">{name}</div>
              <ul className="history__timeline">
                {byTool.get(name)!.map((s) => (
                  <li key={s.id} className="history__entry">
                    <span
                      className="history__dot"
                      style={{
                        background: s.is_outdated
                          ? "var(--yellow)"
                          : s.version
                            ? "var(--green)"
                            : "var(--red)",
                      }}
                    />
                    <span className="history__version">
                      {s.version ?? "missing"}
                    </span>
                    {s.latest && (
                      <span className="muted">→ {s.latest}</span>
                    )}
                    <span className="history__time">
                      {new Date(s.checked_at * 1000).toLocaleString()}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
