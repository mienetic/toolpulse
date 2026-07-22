import { Search, X } from "lucide-react";
import { Dropdown, type DropdownOption } from "./Dropdown";
import {
  CATEGORY_LABELS,
  type SortKey,
  type StatusFilter,
  type ToolCategory,
} from "../types";

export interface ToolFilters {
  query: string;
  category: ToolCategory | "all";
  status: StatusFilter;
  sort: SortKey;
}

interface FiltersBarProps {
  filters: ToolFilters;
  onChange: (next: ToolFilters) => void;
  categories: ToolCategory[];
}

/**
 * Search box + category/status/sort custom dropdowns.
 *
 * `categories` is the list of categories actually present in the current tool
 * set, so the dropdown never offers an empty filter.
 */
export function FiltersBar({ filters, onChange, categories }: FiltersBarProps) {
  const set = <K extends keyof ToolFilters>(key: K, value: ToolFilters[K]) =>
    onChange({ ...filters, [key]: value });

  const hasActive =
    filters.query !== "" ||
    filters.category !== "all" ||
    filters.status !== "all" ||
    filters.sort !== "name";

  const categoryOptions: DropdownOption<ToolCategory | "all">[] = [
    { value: "all", label: "All categories" },
    ...categories.map((c) => ({ value: c, label: CATEGORY_LABELS[c] })),
  ];
  const statusOptions: DropdownOption<StatusFilter>[] = [
    { value: "all", label: "All statuses" },
    { value: "updated", label: "Up to date" },
    { value: "outdated", label: "Updates available" },
    { value: "missing", label: "Not installed" },
    { value: "multi_version", label: "Multiple versions" },
  ];
  const sortOptions: DropdownOption<SortKey>[] = [
    { value: "name", label: "Sort: Name" },
    { value: "status", label: "Sort: Status" },
    { value: "category", label: "Sort: Category" },
  ];

  return (
    <div className="filters">
      <div className="filters__search">
        <Search size={14} />
        <input
          type="text"
          placeholder="Search tools…"
          value={filters.query}
          onChange={(e) => set("query", e.target.value)}
        />
        {filters.query && (
          <button
            className="filters__clear"
            onClick={() => set("query", "")}
            title="Clear"
          >
            <X size={13} />
          </button>
        )}
      </div>

      <Dropdown
        value={filters.category}
        options={categoryOptions}
        onChange={(v) => set("category", v)}
        title="Filter by category"
      />
      <Dropdown
        value={filters.status}
        options={statusOptions}
        onChange={(v) => set("status", v)}
        title="Filter by status"
      />
      <Dropdown
        value={filters.sort}
        options={sortOptions}
        onChange={(v) => set("sort", v)}
        title="Sort by"
      />

      {hasActive && (
        <button
          className="filters__reset"
          onClick={() =>
            onChange({ query: "", category: "all", status: "all", sort: "name" })
          }
        >
          Reset
        </button>
      )}
    </div>
  );
}

/** Apply the filters + sort to a tool list. */
export function applyFilters(
  tools: import("../types").ToolStatus[],
  filters: ToolFilters,
): import("../types").ToolStatus[] {
  const q = filters.query.trim().toLowerCase();
  let out = tools.filter((t) => {
    if (q) {
      const haystack = `${t.name} ${t.display_name}`.toLowerCase();
      if (!haystack.includes(q)) return false;
    }
    if (filters.category !== "all" && t.category !== filters.category) return false;
    if (filters.status !== "all") {
      const installs = t.installations ?? [];
      const multi = installs.length > 1;
      const missing = !t.installed_version;
      const outdated = t.is_outdated;
      const updated = t.installed_version && !outdated;
      switch (filters.status) {
        case "updated":
          if (!updated) return false;
          break;
        case "outdated":
          if (!outdated) return false;
          break;
        case "missing":
          if (!missing) return false;
          break;
        case "multi_version":
          if (!multi) return false;
          break;
      }
    }
    return true;
  });

  out = out.sort((a, b) => {
    switch (filters.sort) {
      case "name":
        return a.display_name.localeCompare(b.display_name);
      case "category":
        return a.category.localeCompare(b.category) || a.display_name.localeCompare(b.display_name);
      case "status": {
        // Outdated first, then missing, then the rest.
        const rank = (t: import("../types").ToolStatus) =>
          t.is_outdated ? 0 : !t.installed_version ? 1 : 2;
        return rank(a) - rank(b) || a.display_name.localeCompare(b.display_name);
      }
    }
  });
  return out;
}
