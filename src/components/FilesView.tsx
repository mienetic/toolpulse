import { useMemo, useState } from "react";
import { FileCode, RefreshCw, Search } from "lucide-react";
import {
  ALL_LANGUAGES,
  LANGUAGE_META,
  formatBytes,
  type SourceFile,
  type SourceLanguage,
} from "../types";

interface FilesViewProps {
  files: SourceFile[];
  scanning: boolean;
  onScan: (languages: SourceLanguage[]) => void;
}

/**
 * Standalone source-file scanner view.
 *
 * Scan state lives in the parent (`useFileScan`) so it survives tab switches.
 */
export function FilesView({ files, scanning, onScan }: FilesViewProps) {
  const [query, setQuery] = useState("");
  const [enabledLangs, setEnabledLangs] = useState<Set<SourceLanguage>>(
    new Set(ALL_LANGUAGES),
  );

  function runScan() {
    onScan([...enabledLangs]);
  }

  function toggleLang(lang: SourceLanguage) {
    setEnabledLangs((prev) => {
      const next = new Set(prev);
      if (next.has(lang)) next.delete(lang);
      else next.add(lang);
      return next;
    });
  }

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return files
      .filter((f) => {
        if (enabledLangs.size < ALL_LANGUAGES.length && !enabledLangs.has(f.language))
          return false;
        if (q && !`${f.name} ${f.path}`.toLowerCase().includes(q)) return false;
        return true;
      })
      .sort((a, b) => a.path.localeCompare(b.path));
  }, [files, query, enabledLangs]);

  const byLang = useMemo(() => {
    const counts = new Map<SourceLanguage, number>();
    for (const f of filtered) {
      counts.set(f.language, (counts.get(f.language) ?? 0) + 1);
    }
    return counts;
  }, [filtered]);

  return (
    <div className="files-view">
      <div className="files-view__header">
        <div className="files-view__title">
          <FileCode size={16} />
          <span>Source Files</span>
        </div>
        <button
          className="btn-primary"
          onClick={runScan}
          disabled={scanning || enabledLangs.size === 0}
        >
          <RefreshCw size={14} className={scanning ? "spin" : ""} />
          <span>{scanning ? "Scanning…" : "Scan files"}</span>
        </button>
      </div>


      {/* Language chips (toggle). */}
      <div className="files-view__langs">
        {ALL_LANGUAGES.map((lang) => (
          <button
            key={lang}
            className="lang-chip"
            data-active={enabledLangs.has(lang)}
            style={{
              borderColor: enabledLangs.has(lang)
                ? LANGUAGE_META[lang].color
                : undefined,
              color: enabledLangs.has(lang) ? LANGUAGE_META[lang].color : undefined,
            }}
            onClick={() => toggleLang(lang)}
          >
            {LANGUAGE_META[lang].label}
            {byLang.has(lang) && (
              <span className="lang-chip__count">{byLang.get(lang)}</span>
            )}
          </button>
        ))}
      </div>

      {files.length > 0 && (
        <div className="files-view__search">
          <Search size={14} />
          <input
            type="text"
            placeholder="Search files…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
      )}

      {scanning ? (
        <div className="scan-progress">
          <RefreshCw size={20} className="spin" />
          <div className="scan-progress__text">
            <div className="scan-progress__title">Scanning source files…</div>
            <div className="muted">
              {files.length > 0
                ? `Found ${files.length} files so far`
                : "Walking directories…"}
            </div>
          </div>
        </div>
      ) : files.length === 0 ? (
        <div className="empty">
          No files scanned yet. Pick languages and click "Scan files".
        </div>
      ) : (
        <ul className="files-view__list">
          {filtered.map((f) => {
            const meta = LANGUAGE_META[f.language];
            return (
              <li key={f.path} className="file-row">
                <span className="file-row__lang" style={{ color: meta.color }}>
                  {meta.label}
                </span>
                <span className="file-row__name">{f.name}</span>
                <span className="file-row__size">{formatBytes(f.size_bytes)}</span>
                <span className="file-row__path" title={f.path}>
                  {f.path}
                </span>
              </li>
            );
          })}
          {filtered.length === 0 && (
            <li className="file-row">
              <span className="muted">
                {scanning ? "Searching…" : "No files match the current filters."}
              </span>
            </li>
          )}
        </ul>
      )}
    </div>
  );
}
