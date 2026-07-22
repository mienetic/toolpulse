import { useEffect, useState } from "react";
import { Layers } from "lucide-react";

/**
 * Inline SVG loader so the icon inherits the wrapper's `color` via
 * `currentColor`. Falls back to a generic icon while loading.
 *
 * Source SVGs are trusted local assets we ship; we inject the markup directly
 * rather than rendering through `<img>` (which can't be tinted).
 */
export function InlineSvg({ path, size = 18 }: { path: string; size?: number }) {
  const [svg, setSvg] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    fetch(path)
      .then((r) => r.text())
      .then((text) => {
        if (alive) setSvg(text);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [path]);
  if (!svg) return <Layers size={size} />;
  return (
    <span
      style={{ width: size, height: size, display: "inline-flex" }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
