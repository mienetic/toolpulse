import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { X, Terminal as TerminalIcon } from "lucide-react";
import { api } from "../lib/api";
import type { TerminalLine } from "../types";

interface TerminalPanelProps {
  /** Filter tag this panel listens for, e.g. "tool:node" or "pkg:npm:typescript". */
  tag: string;
  /** Title shown in the panel header. */
  title: string;
  /** Called when the user closes the panel. */
  onClose: () => void;
}

interface DisplayLine {
  id: number;
  text: string;
  className: string;
}

/**
 * Streaming terminal output panel.
 *
 * Subscribes to `toolpulse://terminal` events whose `tag` matches and renders
 * each line in monospace, colored by stream (stdout = default, stderr = red,
 * status = cyan, done-success = green, done-error = red). Auto-scrolls to the
 * bottom on new output.
 */
export function TerminalPanel({ tag, title, onClose }: TerminalPanelProps) {
  const [lines, setLines] = useState<DisplayLine[]>([]);
  const [done, setDone] = useState<boolean | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const idRef = useRef(0);

  useEffect(() => {
    const unlisten = listen<{ tag: string; line: TerminalLine }>(
      "toolpulse://terminal",
      (e) => {
        if (e.payload.tag !== tag) return;
        const line = e.payload.line;
        const id = idRef.current++;
        if (line.kind === "output") {
          setLines((prev) => [
            ...prev,
            {
              id,
              text: line.text,
              className:
                line.stream === "stderr" ? "term-line term-line--err" : "term-line",
            },
          ]);
        } else if (line.kind === "status") {
          setLines((prev) => [
            ...prev,
            { id, text: line.text, className: "term-line term-line--status" },
          ]);
        } else if (line.kind === "done") {
          setDone(line.success);
          setLines((prev) => [
            ...prev,
            {
              id,
              text: line.message,
              className: line.success
                ? "term-line term-line--ok"
                : "term-line term-line--err",
            },
          ]);
        }
      },
    );
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [tag]);

  // Auto-scroll to bottom whenever new lines arrive.
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines]);

  return (
    <div className="terminal" data-done={done === null ? undefined : done}>
      <div className="terminal__header">
        <TerminalIcon size={13} />
        <span className="terminal__title">{title}</span>
        <span className="terminal__tag">{tag}</span>
        {done === null ? (
          <button
            className="terminal__cancel"
            onClick={() => api.cancelRun().catch(() => {})}
          >
            Cancel
          </button>
        ) : (
          <span
            className="terminal__result"
            style={{ color: done ? "var(--green)" : "var(--red)" }}
          >
            {done ? "✓ Success" : "✗ Failed"}
          </span>
        )}
        <button className="terminal__close" onClick={onClose} title="Close">
          <X size={14} />
        </button>
      </div>
      <div className="terminal__body" ref={scrollRef}>
        {lines.length === 0 ? (
          <div className="term-line term-line--muted">Waiting for output…</div>
        ) : (
          lines.map((l) => (
            <div key={l.id} className={l.className}>
              {l.text}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
