import { Bell, CheckCircle2, Package, XCircle } from "lucide-react";
import type { DashboardSummary } from "../types";

interface DashboardProps {
  summary: DashboardSummary | null;
  lastChecked: number | null;
  loading: boolean;
}

export function Dashboard({ summary, lastChecked, loading }: DashboardProps) {
  if (!summary) {
    return (
      <div className="dashboard">
        <div className="muted">Loading summary…</div>
      </div>
    );
  }

  const upToDate = summary.installed - summary.outdated;
  const cards = [
    {
      label: "Total tools",
      value: summary.total,
      color: "var(--accent)",
      Icon: Package,
    },
    {
      label: "Up to date",
      value: upToDate,
      color: "var(--green)",
      Icon: CheckCircle2,
    },
    {
      label: "Updates available",
      value: summary.outdated,
      color: "var(--yellow)",
      Icon: Bell,
    },
    {
      label: "Not installed",
      value: summary.missing,
      color: "var(--red)",
      Icon: XCircle,
    },
  ];

  return (
    <div className="dashboard">
      <div className="dashboard__cards">
        {cards.map((c) => (
          <div key={c.label} className="summary-card">
            <div className="summary-card__icon" style={{ color: c.color }}>
              <c.Icon size={18} />
            </div>
            <div className="summary-card__text">
              <div className="summary-card__value">{c.value}</div>
              <div className="summary-card__label">{c.label}</div>
            </div>
          </div>
        ))}
      </div>
      <div className="dashboard__meta">
        {loading ? (
          <span className="muted">Checking…</span>
        ) : lastChecked ? (
          <span className="muted">
            Last checked {new Date(lastChecked).toLocaleTimeString()}
          </span>
        ) : (
          <span className="muted">Not checked yet</span>
        )}
      </div>
    </div>
  );
}
