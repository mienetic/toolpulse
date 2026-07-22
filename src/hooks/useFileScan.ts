import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { SourceFile, SourceLanguage } from "../types";

/**
 * Source-file scan state that survives tab switches.
 *
 * Like `useProjectScan`, the results live in a ref so switching tabs and
 * coming back keeps the last scan intact.
 */
export function useFileScan() {
  const filesRef = useRef<SourceFile[]>([]);
  const scanningRef = useRef(false);
  const [, setTick] = useState(0);
  const force = useCallback(() => setTick((t) => t + 1), []);

  useEffect(() => {
    if (!api.isTauri) return;
    let unFile: ((fn: () => void) => void) | null = null;
    let unDone: ((fn: () => void) => void) | null = null;
    listen<SourceFile>("toolpulse://source-file", (e) => {
      filesRef.current = [...filesRef.current, e.payload];
      force();
    })
      .then((fn) => (unFile = fn))
      .catch(() => {});
    listen<number>("toolpulse://source-files-done", () => {
      scanningRef.current = false;
      force();
    })
      .then((fn) => (unDone = fn))
      .catch(() => {});
    return () => {
      unFile?.(() => {});
      unDone?.(() => {});
    };
  }, [force]);

  const scan = useCallback(
    async (languages: SourceLanguage[]) => {
      scanningRef.current = true;
      filesRef.current = [];
      force();
      try {
        const result = await api.scanSourceFiles(languages);
        if (!api.isTauri) {
          filesRef.current = result;
        }
      } catch {
        // surfaced elsewhere
      }
      if (!api.isTauri) {
        scanningRef.current = false;
      }
      force();
    },
    [force],
  );

  return {
    files: filesRef.current,
    scanning: scanningRef.current,
    scan,
  };
}
