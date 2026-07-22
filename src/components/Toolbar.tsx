import { Bell, BellOff, Moon, RefreshCw, Sun, DownloadCloud, Info } from "lucide-react";

/** GitHub repo URL — update after creating the repo. */
export const GITHUB_REPO = "https://github.com/mienetic/toolpulse";

/** Inline GitHub mark (lucide-react doesn't ship a brand icon). */
function GithubIcon({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/>
    </svg>
  );
}

interface ToolbarProps {
  loading: boolean;
  onRefresh: () => void;
  onToggleTheme: () => void;
  theme: "dark" | "light";
  notificationsEnabled: boolean;
  onToggleNotifications: () => void;
  /** When an update is available, show a badge on the refresh button. */
  updateAvailable?: boolean;
  onUpdate?: () => void;
  onAbout?: () => void;
}

export function Toolbar({
  loading,
  onRefresh,
  onToggleTheme,
  theme,
  notificationsEnabled,
  onToggleNotifications,
  updateAvailable,
  onUpdate,
  onAbout,
}: ToolbarProps) {
  return (
    <div className="toolbar">
      <div className="toolbar__brand">
        <img src="/logo.svg" alt="Toolpulse" className="toolbar__logo" width={28} height={28} />
        <span className="toolbar__title">Toolpulse</span>
        {updateAvailable && (
          <button className="update-badge" onClick={onUpdate} title="Update available — click to install">
            <DownloadCloud size={12} /> Update
          </button>
        )}
      </div>
      <div className="toolbar__actions">
        <button
          className="btn-icon"
          title="About Toolpulse"
          onClick={onAbout}
        >
          <Info size={16} />
        </button>
        <a
          className="btn-icon"
          href={GITHUB_REPO}
          target="_blank"
          rel="noopener noreferrer"
          title="View on GitHub"
        >
          <GithubIcon size={16} />
        </a>
        <button
          className="btn-icon"
          title={notificationsEnabled ? "Notifications on" : "Notifications off"}
          onClick={onToggleNotifications}
          data-active={notificationsEnabled}
        >
          {notificationsEnabled ? <Bell size={16} /> : <BellOff size={16} />}
        </button>
        <button
          className="btn-icon"
          title="Toggle theme"
          onClick={onToggleTheme}
        >
          {theme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
        </button>
        <button
          className="btn-primary"
          onClick={onRefresh}
          disabled={loading}
        >
          <RefreshCw size={15} className={loading ? "spin" : ""} />
          <span>{loading ? "Checking…" : "Refresh"}</span>
        </button>
      </div>
    </div>
  );
}
