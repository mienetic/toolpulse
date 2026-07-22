import { useCallback, useState } from "react";
import { api } from "../lib/api";
import type { ActionKind } from "../types";

interface PendingAction {
  subject: string;
  detail?: string;
  action: ActionKind;
  /** What to run once the user confirms. */
  run: () => Promise<void>;
}

export interface ActiveTerminal {
  tag: string;
  title: string;
}

interface UseTerminalRunResult {
  /** The pending confirmation, or null if none. */
  pending: PendingAction | null;
  /** Terminals currently showing streaming output. */
  terminals: ActiveTerminal[];
  /** Request a confirm + run flow for a tool. */
  requestTool: (
    name: string,
    displayName: string,
    action: ActionKind,
    tag: string,
    onDone: () => void,
  ) => void;
  /** Request a confirm + run flow for a package. */
  requestPackage: (
    tool: string,
    pkg: string,
    action: ActionKind,
    tag: string,
    onDone: () => void,
  ) => void;
  confirm: () => void;
  cancel: () => void;
  closeTerminal: (tag: string) => void;
}

/**
 * Coordinates the confirm → run → terminal-panel lifecycle for install /
 * uninstall / upgrade commands.
 *
 * Centralizing this in a hook keeps App.tsx free of action plumbing and lets
 * any component (tool card, package row) trigger a run via a stable callback.
 */
export function useTerminalRun(): UseTerminalRunResult {
  const [pending, setPending] = useState<PendingAction | null>(null);
  const [terminals, setTerminals] = useState<ActiveTerminal[]>([]);

  const request = useCallback(
    (
      subject: string,
      action: ActionKind,
      title: string,
      run: () => Promise<void>,
    ) => {
      setPending({ subject, action, run, detail: title });
    },
    [],
  );

  const requestTool = useCallback<UseTerminalRunResult["requestTool"]>(
    (name, displayName, action, tag, onDone) => {
      const title = action === "upgrade" ? `Upgrade ${displayName}` : `${displayName}`;
      request(displayName, action, title, async () => {
        setTerminals((prev) =>
          prev.some((t) => t.tag === tag)
            ? prev
            : [...prev, { tag, title: `${action} ${displayName}` }],
        );
        try {
          await api.manageTool(name, action);
        } catch {
          // Error already surfaced via the terminal Done line + notification.
        }
        onDone();
      });
    },
    [request],
  );

  const requestPackage = useCallback<UseTerminalRunResult["requestPackage"]>(
    (tool, pkg, action, tag, onDone) => {
      const title = `${pkg} (${tool})`;
      request(title, action, title, async () => {
        setTerminals((prev) =>
          prev.some((t) => t.tag === tag)
            ? prev
            : [...prev, { tag, title: `${action} ${pkg}` }],
        );
        try {
          await api.managePackage(tool, pkg, action);
        } catch {
          // surfaced via terminal + notification
        }
        onDone();
      });
    },
    [request],
  );

  const confirm = useCallback(() => {
    if (!pending) return;
    const p = pending;
    setPending(null);
    p.run().catch(() => {});
  }, [pending]);

  const cancel = useCallback(() => setPending(null), []);

  const closeTerminal = useCallback((tag: string) => {
    setTerminals((prev) => prev.filter((t) => t.tag !== tag));
  }, []);

  return {
    pending,
    terminals,
    requestTool,
    requestPackage,
    confirm,
    cancel,
    closeTerminal,
  };
}
