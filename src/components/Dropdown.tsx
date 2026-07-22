import { useEffect, useRef, useState } from "react";
import { ChevronDown, Check } from "lucide-react";

export interface DropdownOption<T extends string> {
  value: T;
  label: string;
}

interface DropdownProps<T extends string> {
  value: T;
  options: DropdownOption<T>[];
  onChange: (value: T) => void;
  title?: string;
}

/**
 * Custom dropdown rendered entirely in HTML/React (no native `<select>`),
 * so it matches the app theme on every platform instead of falling back to
 * the OS popup.
 *
 * Closes on outside click or Escape. Keyboard: Up/Down to move, Enter to
 * select, Escape to close.
 */
export function Dropdown<T extends string>({
  value,
  options,
  onChange,
  title,
}: DropdownProps<T>) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const selected = options.find((o) => o.value === value);

  // Close on outside click.
  useEffect(() => {
    if (!open) return;
    function onDown(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  // Reset keyboard cursor when opening.
  useEffect(() => {
    if (open) {
      const idx = options.findIndex((o) => o.value === value);
      setActiveIndex(idx >= 0 ? idx : 0);
    }
  }, [open, options, value]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (!open) {
      if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        setOpen(true);
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, options.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        if (options[activeIndex]) {
          onChange(options[activeIndex].value);
          setOpen(false);
        }
        break;
      case "Escape":
        e.preventDefault();
        setOpen(false);
        break;
    }
  }

  // Scroll the active option into view.
  useEffect(() => {
    if (!open || !listRef.current) return;
    const el = listRef.current.querySelector<HTMLElement>(
      `[data-idx="${activeIndex}"]`,
    );
    el?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, open]);

  return (
    <div
      className="dropdown"
      ref={rootRef}
      title={title}
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      <button
        type="button"
        className="dropdown__trigger"
        data-open={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="dropdown__label">{selected?.label ?? value}</span>
        <ChevronDown size={13} className={open ? "dropdown__chevron--up" : ""} />
      </button>

      {open && (
        <div className="dropdown__menu" ref={listRef} role="listbox">
          {options.map((opt, i) => (
            <button
              key={opt.value}
              type="button"
              role="option"
              data-idx={i}
              aria-selected={opt.value === value}
              className="dropdown__option"
              data-active={i === activeIndex}
              onMouseEnter={() => setActiveIndex(i)}
              onClick={() => {
                onChange(opt.value);
                setOpen(false);
              }}
            >
              <span className="dropdown__option-label">{opt.label}</span>
              {opt.value === value && <Check size={13} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
