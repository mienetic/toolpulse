import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { DiscoveredProject } from "../types";

/**
 * Project scan state that survives tab switches.
 *
 * The scan results live in a ref (not React state) so they persist across
 * mount/unmount cycles when the user switches tabs. A lightweight `tick` state
 * forces a re-render after mutations.
 *
 * Progressive updates arrive via `toolpulse://project` events in Tauri; in the
 * browser fallback we use the resolved promise value directly.
 */
export function useProjectScan() {
  const projectsRef = useRef<DiscoveredProject[]>([]);
  const scanningRef = useRef(false);
  const rootRef = useRef<string | null>(null);
  const [, setTick] = useState(0);
  const force = useCallback(() => setTick((t) => t + 1), []);

  // Subscribe to progressive events once. The listeners are attached on mount
  // and detached on unmount, but the data they write lives in refs that
  // outlive the component.
  useEffect(() => {
    if (!api.isTauri) return;
    let unProject: ((fn: () => void) => void) | null = null;
    let unDone: ((fn: () => void) => void) | null = null;
    listen<DiscoveredProject>("toolpulse://project", (e) => {
      const key = `${e.payload.ecosystem}:${e.payload.path}`;
      if (!projectsRef.current.some((p) => `${p.ecosystem}:${p.path}` === key)) {
        projectsRef.current = [...projectsRef.current, e.payload];
        force();
      }
    })
      .then((fn) => (unProject = fn))
      .catch(() => {});
    listen<number>("toolpulse://scan-done", () => {
      scanningRef.current = false;
      force();
    })
      .then((fn) => (unDone = fn))
      .catch(() => {});
    return () => {
      unProject?.(() => {});
      unDone?.(() => {});
    };
  }, [force]);

  const scanMachine = useCallback(async () => {
    scanningRef.current = true;
    projectsRef.current = [];
    rootRef.current = "whole machine ($HOME + /Applications + /Volumes + /opt)";
    force();
    try {
      const result = await api.scanMachine();
      if (!api.isTauri) {
        projectsRef.current = result;
      }
    } catch {
      // surfaced elsewhere
    }
    if (!api.isTauri) {
      scanningRef.current = false;
    }
    force();
  }, [force]);

  const scanFolder = useCallback(
    async (root?: string) => {
      scanningRef.current = true;
      projectsRef.current = [];
      rootRef.current = root ?? "$HOME";
      force();
      try {
        projectsRef.current = await api.scanProjects(root);
      } catch {
        // surfaced elsewhere
      }
      scanningRef.current = false;
      force();
    },
    [force],
  );

  const pickFolder = useCallback(async () => {
    try {
      const folder = await api.pickFolder();
      if (folder) await scanFolder(folder);
    } catch {
      // ignore
    }
  }, [scanFolder]);

  const removeProject = useCallback(
    (path: string) => {
      projectsRef.current = projectsRef.current.filter((p) => p.path !== path);
      force();
    },
    [force],
  );

  return {
    projects: projectsRef.current,
    scanning: scanningRef.current,
    scanRoot: rootRef.current,
    scanMachine,
    scanFolder,
    pickFolder,
    removeProject,
  };
}
