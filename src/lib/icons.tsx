import { useEffect, useState } from "react";

// Maps tool name -> path of its brand SVG icon (from Simple Icons, CC0).
// When a tool isn't listed, callers fall back to the emoji in `tool.icon`.

const ICON_PATHS: Record<string, string> = {
  node: "/icons/node.svg",
  npm: "/icons/npm.svg",
  yarn: "/icons/yarn.svg",
  pnpm: "/icons/pnpm.svg",
  bun: "/icons/bun.svg",
  deno: "/icons/deno.svg",
  python: "/icons/python.svg",
  pip: "/icons/pip.svg",
  rust: "/icons/rust.svg",
  cargo: "/icons/rust.svg",
  rustup: "/icons/rustup.svg",
  zig: "/icons/zig.svg",
  go: "/icons/go.svg",
  swift: "/icons/swift.svg",
  ruby: "/icons/ruby.svg",
  dotnet: "/icons/dotnet.svg",
  java: "/icons/java.svg",
  php: "/icons/php.svg",
  docker: "/icons/docker.svg",
  brew: "/icons/brew.svg",
  gem: "/icons/gem.svg",
  composer: "/icons/composer.svg",
  nvm: "/icons/nvm.svg",
  pyenv: "/icons/python.svg",
  asdf: "/icons/python.svg",
};

/** Return the brand SVG path for a tool, or `null` to fall back to emoji. */
export function brandIconPath(name: string): string | null {
  return ICON_PATHS[name] ?? null;
}

// Cache SVG source so we only fetch each icon once across the app.
const svgCache = new Map<string, string>();

/**
 * Render a tool's icon as an inline SVG so CSS `color` tints it via
 * `currentColor`. Falls back to the emoji while loading or when no brand
 * icon exists.
 *
 * We inject the fetched SVG markup directly so the `<path>` (which omits a
 * `fill`, defaulting to `currentColor`) picks up the parent's `color`.
 */
export function ToolIcon({
  name,
  emoji,
  size = 22,
}: {
  name: string;
  emoji: string;
  size?: number;
}) {
  const path = brandIconPath(name);
  const [svg, setSvg] = useState<string | null>(() =>
    path ? svgCache.get(path) ?? null : null,
  );

  useEffect(() => {
    if (!path) return;
    if (svgCache.has(path)) {
      setSvg(svgCache.get(path)!);
      return;
    }
    let alive = true;
    fetch(path)
      .then((r) => r.text())
      .then((text) => {
        if (!alive) return;
        svgCache.set(path, text);
        setSvg(text);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [path]);

  if (svg) {
    return (
      <span
        className="tool-icon-svg"
        style={{ width: size, height: size, display: "inline-flex" }}
        // Inline SVG so `currentColor` works. The source is a trusted local
        // asset we ship, never user input.
        dangerouslySetInnerHTML={{ __html: svg }}
      />
    );
  }
  return <span style={{ fontSize: size, lineHeight: 1 }}>{emoji}</span>;
}
