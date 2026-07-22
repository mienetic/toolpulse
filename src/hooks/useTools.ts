import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import type { DashboardSummary, ToolStatus } from "../types";

interface UseToolsResult {
  tools: ToolStatus[];
  loading: boolean;
  error: string | null;
  lastChecked: number | null;
  summary: DashboardSummary | null;
  refresh: () => Promise<void>;
  refreshOne: (name: string) => Promise<void>;
}

/**
 * Owns the tool-status list and drives scans.
 *
 * Scans run on mount (auto-check) and on explicit `refresh()`. A ref guards
 * against overlapping scans so a double-click can't spawn two network bursts.
 */
export function useTools(autoCheck: boolean): UseToolsResult {
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastChecked, setLastChecked] = useState<number | null>(null);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const inFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setLoading(true);
    setError(null);
    try {
      const result = await api.checkAllTools();
      setTools(result);
      setLastChecked(Date.now());
      setSummary(await api.dashboardSummary(result));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
      inFlight.current = false;
    }
  }, []);

  const refreshOne = useCallback(async (name: string) => {
    try {
      const updated = await api.checkTool(name);
      setTools((prev) => {
        const next = prev.some((t) => t.name === name)
          ? prev.map((t) => (t.name === name ? updated : t))
          : [...prev, updated];
        api.dashboardSummary(next).then(setSummary).catch(() => {});
        return next;
      });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    if (autoCheck) refresh();
    // We intentionally only auto-check once on mount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { tools, loading, error, lastChecked, summary, refresh, refreshOne };
}
