import type { DiscoveredProject, ProjectEcosystem } from "../types";

/**
 * A tree node: either a folder containing children, or a leaf project.
 *
 * We build this from a flat list of project paths by splitting each path into
 * segments and nesting. Intermediate folders that contain only one child path
 * are collapsed into a single breadcrumb so the tree stays shallow.
 */
export type TreeNode =
  | {
      kind: "folder";
      /** Display name of this folder segment. */
      name: string;
      /** Full path prefix this folder represents. */
      path: string;
      children: TreeNode[];
      // Aggregated stats across all descendant projects.
      projectCount: number;
      sizeBytes: number;
      outdatedCount: number;
      ecosystems: Set<ProjectEcosystem>;
    }
  | {
      kind: "project";
      name: string;
      project: DiscoveredProject;
    };

/**
 * Build a tree from a flat project list.
 *
 * Folders along the path that contain exactly one chain of children get
 * collapsed into one node (joined with "/") so deeply-nested solo paths
 * don't produce tall thin trees.
 */
export function buildTree(projects: DiscoveredProject[]): TreeNode[] {
  if (projects.length === 0) return [];

  interface TrieNode {
    name: string;
    path: string;
    children: Map<string, TrieNode>;
    project: DiscoveredProject | null;
  }

  const root: TrieNode = {
    name: "",
    path: "",
    children: new Map(),
    project: null,
  };

  for (const p of projects) {
    // Show the project's parent folder as its display name, since a manifest
    // often lives in a subdirectory (e.g. `toolpulse/src-tauri/Cargo.toml`)
    // while the user thinks of the project as `toolpulse`.
    const display = parentFolderName(p.path);
    // Strip system-root path segments so the tree starts at the first folder
    // that's actually meaningful (e.g. "projects", "work"), never /Users/<name>
    // or /usr/local or /Applications.
    const cleaned = stripSystemPrefix(p.path);
    const segments = cleaned.split("/").filter(Boolean);
    let cur = root;
    let acc = "";
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      acc = acc ? `${acc}/${seg}` : seg;
      let child = cur.children.get(seg);
      if (!child) {
        child = { name: seg, path: acc, children: new Map(), project: null };
        cur.children.set(seg, child);
      }
      cur = child;
    }
    cur.project = { ...p, name: display };
  }

  // Convert the trie into TreeNode[], collapsing single-child non-project
  // folders into breadcrumb names.
  function convert(node: TrieNode): TreeNode[] {
    const out: TreeNode[] = [];
    for (const child of node.children.values()) {
      out.push(...convertNode(child));
    }
    return out;
  }

  function convertNode(node: TrieNode): TreeNode[] {
    // If this node is a project AND has no children, emit a single leaf.
    if (node.project && node.children.size === 0) {
      return [
        {
          kind: "project",
          name: node.project.name,
          project: node.project,
        },
      ];
    }

    // Collapse chains of single-child folders (no project at each step) into
    // one breadcrumb to keep the tree shallow.
    let collapsed = node;
    let nameParts = [node.name];
    while (
      collapsed.project === null &&
      collapsed.children.size === 1
    ) {
      const only = [...collapsed.children.values()][0];
      // Don't collapse if the only child is a project leaf — keep it as the
      // project's own folder.
      if (only.project && only.children.size === 0) break;
      collapsed = only;
      nameParts.push(only.name);
    }

    const children = convert(collapsed);
    // Aggregate stats across all descendant projects.
    const { projectCount, sizeBytes, outdatedCount, ecosystems } =
      folderStats(children);

    // If this collapsed folder contains exactly one project and nothing else,
    // just emit the project directly (folder == project dir).
    if (projectCount === 1 && children.length === 1 && children[0].kind === "project") {
      return children;
    }

    return [
      {
        kind: "folder",
        name: nameParts.join("/"),
        path: collapsed.path,
        children,
        projectCount,
        sizeBytes,
        outdatedCount,
        ecosystems,
      },
    ];
  }

  return convert(root);
}

// Note: the collectStats approach above is replaced by direct folder field
// reads in the component — folders already aggregate during construction.
// Re-implement cleanly:
export function folderStats(nodes: TreeNode[]) {
  let projectCount = 0;
  let sizeBytes = 0;
  let outdatedCount = 0;
  const ecosystems = new Set<ProjectEcosystem>();
  function walk(ns: TreeNode[]) {
    for (const n of ns) {
      if (n.kind === "project") {
        projectCount++;
        sizeBytes += n.project.size_bytes;
        outdatedCount += n.project.outdated_count ?? 0;
        ecosystems.add(n.project.ecosystem);
      } else {
        walk(n.children);
      }
    }
  }
  walk(nodes);
  return { projectCount, sizeBytes, outdatedCount, ecosystems };
}

/**
 * Strip well-known system-root path segments from an absolute path so the
 * project tree starts at the first folder the user actually cares about.
 *
 * Handles `/Users/<name>`, `/home/<name>`, `/usr/local`, `/Applications`,
 * `/opt`, `/Volumes/<vol>`, and `/private/<...>`. Works on any machine —
 * the username is matched generically, not hardcoded.
 */
function stripSystemPrefix(path: string): string {
  // Remove leading slash, then drop system-root segments from the front.
  const segments = path.replace(/^\//, "").split("/").filter(Boolean);
  const out: string[] = [];
  let i = 0;
  // Skip /Users/<name> or /home/<name>.
  if ((segments[0] === "Users" || segments[0] === "home") && segments.length > 2) {
    i = 2; // skip "Users" + username
  } else if (segments[0] === "usr" && segments[1] === "local") {
    i = 2;
  } else if (segments[0] === "Applications" || segments[0] === "opt") {
    i = 1;
  } else if (segments[0] === "Volumes" && segments.length > 1) {
    i = 2; // skip "Volumes" + volume name
  } else if (segments[0] === "private") {
    i = 1;
  }
  for (; i < segments.length; i++) {
    out.push(segments[i]);
  }
  return out.join("/");
}

/**
 * Return the parent folder name of a path — the human-friendly project label.
 *
 * e.g. `/Users/apple/projects/toolpulse/src-tauri` → `toolpulse`.
 * We walk up until we find a folder that doesn't look like a boilerplate
 * subdirectory (`src`, `src-tauri`, `app`, `server`, etc.).
 */
function parentFolderName(path: string): string {
  const segments = path.split("/").filter(Boolean);
  if (segments.length === 0) return path;
  // Boilerplate subdirs that shouldn't be the project's display name.
  const boilerplate = new Set([
    "src", "src-tauri", "app", "server", "api", "web", "client", "backend",
    "frontend", "lib", "cmd", "internal", "pkg", "public", "dist", "build",
  ]);
  for (let i = segments.length - 1; i >= 0; i--) {
    if (!boilerplate.has(segments[i].toLowerCase())) {
      return segments[i];
    }
  }
  return segments[segments.length - 1];
}
