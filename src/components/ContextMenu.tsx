import { useEffect, useRef, useState } from "react";

export interface ContextMenuItem {
  /** Stable id. */
  id: string;
  /** Label shown to the user. */
  label: string;
  /** Optional icon node. */
  icon?: React.ReactNode;
  /** Run when clicked. */
  onSelect?: () => void;
  /// Render a separator before this item.
  separator?: boolean;
  /// Disable the item (greyed out).
  disabled?: boolean;
  /// Destructive actions are tinted red.
  danger?: boolean;
}

interface ContextMenuProps {
  open: boolean;
  /// Screen coordinates (clientX/clientY) to anchor the menu at.
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

/**
 * A floating context menu anchored at a screen position.
 *
 * Repositions to stay within the viewport, closes on outside click / Escape,
 * and supports keyboard navigation (Up/Down/Enter). Used for right-click
 * actions on project tree rows.
 */
export function ContextMenu({ open, x, y, items, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);

  // Close on outside click or Escape.
  useEffect(() => {
    if (!open) return;
    function onDown(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  useEffect(() => {
    if (open) {
      const first = items.findIndex((i) => !i.disabled && !i.separator);
      setActiveIndex(first >= 0 ? first : 0);
    }
  }, [open, items]);

  if (!open) return null;

  // Keep the menu inside the viewport.
  const maxX = window.innerWidth - 220;
  const maxY = window.innerHeight - items.length * 34 - 20;
  const left = Math.min(x, maxX);
  const top = Math.min(y, maxY);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => {
        let n = i;
        do {
          n = (n + 1) % items.length;
        } while ((items[n].disabled || items[n].separator) && n !== i);
        return n;
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => {
        let n = i;
        do {
          n = (n - 1 + items.length) % items.length;
        } while ((items[n].disabled || items[n].separator) && n !== i);
        return n;
      });
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = items[activeIndex];
      if (item && !item.disabled && !item.separator) {
        item.onSelect?.();
        onClose();
      }
    }
  }

  return (
    <div
      ref={ref}
      className="context-menu"
      style={{ left, top }}
      role="menu"
      tabIndex={-1}
      onKeyDown={onKeyDown}
    >
      {items.map((item, i) => (
        <div key={item.id}>
          {item.separator && <div className="context-menu__sep" />}
          <button
            role="menuitem"
            className="context-menu__item"
            data-active={i === activeIndex}
            data-danger={item.danger}
            disabled={item.disabled}
            onMouseEnter={() => !item.separator && setActiveIndex(i)}
            onClick={() => {
              if (!item.disabled && !item.separator) {
                item.onSelect?.();
                onClose();
              }
            }}
          >
            {item.icon && <span className="context-menu__icon">{item.icon}</span>}
            <span>{item.label}</span>
          </button>
        </div>
      ))}
    </div>
  );
}
