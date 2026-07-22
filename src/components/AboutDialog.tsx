import { X, Heart, DownloadCloud } from "lucide-react";
import { GITHUB_REPO } from "./Toolbar";

/** Inline GitHub mark (lucide-react has no brand icon). */
function GithubMark({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/>
    </svg>
  );
}

interface AboutDialogProps {
  open: boolean;
  onClose: () => void;
  appVersion: string;
  updateAvailable: boolean;
  updateVersion: string | null;
  onUpdate: () => void;
}

/**
 * About dialog — shows app identity, version, links, and credits.
 *
 * Doubles as an update prompt: when an update is available, a prominent
 * "Update now" button appears at the top.
 */
export function AboutDialog({
  open,
  onClose,
  appVersion,
  updateAvailable,
  updateVersion,
  onUpdate,
}: AboutDialogProps) {
  if (!open) return null;
  return (
    <div className="overlay-backdrop" onClick={onClose}>
      <div className="about-dialog" onClick={(e) => e.stopPropagation()}>
        <button className="about-dialog__close" onClick={onClose}>
          <X size={16} />
        </button>

        <div className="about-dialog__logo">
          <img src="/logo.svg" alt="Toolpulse" width={64} height={64} />
        </div>

        <h2 className="about-dialog__title">Toolpulse</h2>
        <p className="about-dialog__version">v{appVersion}</p>
        <p className="about-dialog__tagline">
          The desktop dashboard for every dev tool on your machine.
        </p>

        {updateAvailable && (
          <button className="about-dialog__update" onClick={onUpdate}>
            <DownloadCloud size={15} />
            <span>Update to v{updateVersion}</span>
          </button>
        )}

        <div className="about-dialog__links">
          <a
            className="about-dialog__link"
            href={GITHUB_REPO}
            target="_blank"
            rel="noopener noreferrer"
          >
            <GithubMark size={14} /> GitHub
          </a>
          <a
            className="about-dialog__link"
            href={`${GITHUB_REPO}/releases`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <DownloadCloud size={14} /> Releases
          </a>
          <a
            className="about-dialog__link"
            href={`${GITHUB_REPO}/issues/new`}
            target="_blank"
            rel="noopener noreferrer"
          >
            Report issue
          </a>
        </div>

        <div className="about-dialog__features">
          <span>27+ tools monitored</span>
          <span>8 ecosystems</span>
          <span>11 languages scanned</span>
          <span>macOS · Windows · Linux</span>
        </div>

        <p className="about-dialog__credits">
          Built with <Heart size={11} style={{ display: "inline", color: "var(--red)" }} /> using
          Tauri 2 · React · Rust
        </p>
        <p className="about-dialog__license">MIT License © 2026 Toolpulse Contributors</p>
      </div>
    </div>
  );
}
