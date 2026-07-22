import { AlertTriangle } from "lucide-react";
import type { ActionKind } from "../types";

interface ConfirmDialogProps {
  open: boolean;
  action: ActionKind;
  subject: string;
  detail?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

const ACTION_META: Record<
  ActionKind,
  { label: string; verb: string; danger: boolean }
> = {
  install: { label: "Install", verb: "install", danger: false },
  uninstall: { label: "Uninstall", verb: "uninstall", danger: true },
  upgrade: { label: "Upgrade", verb: "upgrade", danger: false },
};

/**
 * Modal confirmation dialog shown before any system-mutating command runs.
 *
 * Uninstall actions are flagged as dangerous (red button) since they remove
 * software; install/upgrade use the accent color.
 */
export function ConfirmDialog({
  open,
  action,
  subject,
  detail,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  if (!open) return null;
  const meta = ACTION_META[action];

  return (
    <div className="overlay-backdrop" onClick={onCancel}>
      <div
        className="confirm-dialog"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="confirm-dialog__icon">
          {meta.danger ? (
            <AlertTriangle size={20} style={{ color: "var(--red)" }} />
          ) : (
            <AlertTriangle size={20} style={{ color: "var(--yellow)" }} />
          )}
        </div>
        <div className="confirm-dialog__body">
          <div className="confirm-dialog__title">
            {meta.label} {subject}?
          </div>
          {detail && <div className="confirm-dialog__detail">{detail}</div>}
          <div className="confirm-dialog__warning">
            This will run a real command on your system.
          </div>
        </div>
        <div className="confirm-dialog__actions">
          <button className="btn-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button
            className={meta.danger ? "btn-danger" : "btn-primary"}
            onClick={onConfirm}
          >
            {meta.label}
          </button>
        </div>
      </div>
    </div>
  );
}
