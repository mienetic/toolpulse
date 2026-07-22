import { useState } from "react";
import {
  Folder,
  ChevronDown,
  ChevronUp,
  AlertTriangle,
  Layers,
} from "lucide-react";
import { brandIconPath } from "../lib/icons";
import { InlineSvg } from "./InlineSvg";
import {
  ECOSYSTEM_META,
  formatBytes,
  type DetectedIde,
  type DiscoveredProject,
  type ProjectDependency,
} from "../types";
import type { TreeNode } from "../lib/projectTree";

interface ProjectTreeProps {
  tree: TreeNode[];
  expandedProjects: Record<string, boolean>;
  deps: Record<string, ProjectDependency[]>;
  loadingDeps: string | null;
  ides: DetectedIde[];
  onToggleProject: (p: DiscoveredProject) => void;
  // Context-menu callbacks. `target` is the path the menu was opened on;
  // `project` (when present) carries ecosystem for deps-folder logic.
  onContextAction: (
    action: ContextAction,
    path: string,
    project: DiscoveredProject | null,
  ) => void;
}

export type ContextAction =
  | "open-folder"
  | "open-terminal"
  | "open-deps"
  | "copy-path"
  | "copy-name"
  | "trash"
  | { ide: DetectedIde };

export function ProjectTree({
  tree,
  expandedProjects,
  deps,
  loadingDeps,
  ides,
  onToggleProject,
  onContextAction,
}: ProjectTreeProps) {
  return (
    <ul className="tree" role="tree">
      {tree.map((node) => (
        <TreeRow
          key={node.kind === "folder" ? node.path : node.project.path}
          node={node}
          depth={0}
          expandedProjects={expandedProjects}
          deps={deps}
          loadingDeps={loadingDeps}
          ides={ides}
          onToggleProject={onToggleProject}
          onContextAction={onContextAction}
        />
      ))}
    </ul>
  );
}

function TreeRow({
  node,
  depth,
  expandedProjects,
  deps,
  loadingDeps,
  ides,
  onToggleProject,
  onContextAction,
}: {
  node: TreeNode;
  depth: number;
  expandedProjects: Record<string, boolean>;
  deps: Record<string, ProjectDependency[]>;
  loadingDeps: string | null;
  ides: DetectedIde[];
  onToggleProject: (p: DiscoveredProject) => void;
  onContextAction: (
    action: ContextAction,
    path: string,
    project: DiscoveredProject | null,
  ) => void;
}) {
  if (node.kind === "project") {
    return (
      <ProjectLeaf
        project={node.project}
        depth={depth}
        expanded={!!expandedProjects[node.project.path]}
        deps={deps[node.project.path]}
        loading={loadingDeps === node.project.path}
        onToggle={() => onToggleProject(node.project)}
      />
    );
  }
  return (
    <FolderNode
      folder={node}
      depth={depth}
      expandedProjects={expandedProjects}
      deps={deps}
      loadingDeps={loadingDeps}
      ides={ides}
      onToggleProject={onToggleProject}
      onContextAction={(a) => onContextAction(a, node.path, null)}
    />
  );
}

function FolderNode({
  folder,
  depth,
  expandedProjects,
  deps,
  loadingDeps,
  ides,
  onToggleProject,
  onContextAction,
}: {
  folder: Extract<TreeNode, { kind: "folder" }>;
  depth: number;
  expandedProjects: Record<string, boolean>;
  deps: Record<string, ProjectDependency[]>;
  loadingDeps: string | null;
  ides: DetectedIde[];
  onToggleProject: (p: DiscoveredProject) => void;
  onContextAction: (action: ContextAction) => void;
}) {
  const [open, setOpen] = useState(depth < 1);
  return (
    <li role="treeitem" className="tree__folder">
      <button
        className="tree__folder-head"
        style={{ paddingLeft: 12 + depth * 16 }}
        onClick={() => setOpen((v) => !v)}
      >
        <Folder size={14} style={{ color: "var(--accent)", flexShrink: 0 }} />
        <span className="tree__folder-name">{folder.name}</span>
        <span className="tree__folder-ecos">
          {[...folder.ecosystems].map((e) => (
            <span
              key={e}
              className="tree__eco-badge"
              style={{ color: ECOSYSTEM_META[e].color }}
            >
              {ECOSYSTEM_META[e].label}
            </span>
          ))}
        </span>
        <span className="tree__folder-count">{folder.projectCount} projects</span>
        {folder.outdatedCount > 0 && (
          <span className="tree__folder-outdated">
            <AlertTriangle size={11} /> {folder.outdatedCount}
          </span>
        )}
        <span className="tree__folder-size">{formatBytes(folder.sizeBytes)}</span>
        {open ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
      </button>
      {open && (
        <ul role="group" className="tree__children">
          {folder.children.map((child) => (
            <TreeRow
              key={child.kind === "folder" ? child.path : child.project.path}
              node={child}
              depth={depth + 1}
              expandedProjects={expandedProjects}
              deps={deps}
              loadingDeps={loadingDeps}
              ides={ides}
              onToggleProject={onToggleProject}
              onContextAction={onContextAction}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

function ProjectLeaf({
  project,
  depth,
  expanded,
  deps,
  loading,
  onToggle,
}: {
  project: DiscoveredProject;
  depth: number;
  expanded: boolean;
  deps?: ProjectDependency[];
  loading: boolean;
  onToggle: () => void;
}) {
  const meta = ECOSYSTEM_META[project.ecosystem];
  const iconPath = brandIconPath(meta.icon);
  return (
    <li role="treeitem" className="tree__project">
      <div
        className="tree__project-head"
        style={{ paddingLeft: 12 + depth * 16 }}
        onClick={onToggle}
      >
        <span className="tree__project-icon" style={{ color: meta.color }}>
          {iconPath ? (
            <InlineSvg path={iconPath} size={16} />
          ) : (
            <Layers size={14} />
          )}
        </span>
        <span className="tree__project-name">{project.name}</span>
        <span className="tree__project-eco" style={{ color: meta.color }}>
          {meta.label}
        </span>
        <span className="tree__project-deps">{project.dependency_count} deps</span>
        {(project.outdated_count ?? 0) > 0 && (
          <span className="tree__project-outdated">
            <AlertTriangle size={11} /> {project.outdated_count}
          </span>
        )}
        <span className="tree__project-size">
          {formatBytes(project.size_bytes)}
        </span>
        {!project.is_real_project && (
          <span className="tree__project-tag" title="Loose manifest, not a full project">
            loose
          </span>
        )}
        {expanded ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
      </div>
      <div className="tree__project-path" title={project.path}>
        {project.path}
      </div>
      {expanded && (
        <div className="tree__project-deps-panel">
          {loading ? (
            <div className="muted">Loading dependencies…</div>
          ) : deps && deps.length > 0 ? (
            <ul className="dep-list">
              {deps.map((d, i) => (
                <li
                  key={`${d.name}-${i}`}
                  className="dep-list__item"
                  data-outdated={d.is_outdated === true}
                >
                  <span className="dep-list__name">{d.name}</span>
                  <span className="dep-list__version">{d.version ?? "—"}</span>
                  {d.is_outdated === true && d.latest && (
                    <span className="dep-list__latest">→ {d.latest}</span>
                  )}
                  {d.is_outdated === true && (
                    <AlertTriangle size={11} style={{ color: "var(--yellow)" }} />
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <div className="muted">No dependencies parsed.</div>
          )}
        </div>
      )}
    </li>
  );
}
