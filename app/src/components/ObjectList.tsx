import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, ArrowUpDown, Check, Download, FileText, FolderOpen, Inbox, Loader2, RefreshCw } from "lucide-react";
import { api, BackendError, isTauri } from "@/api/client";
import type { DocFormat, ListItem, ObjectType } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useToast } from "@/components/ui/toast";
import { groupIcon } from "@/lib/icons";

interface Props {
  type: ObjectType;
  search: string;
  onOpen: (item: ListItem) => void;
}

type SortKey = "name" | "odataType" | "id";

async function pickDirectory(): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ directory: true, multiple: false, title: "Choose export folder" });
    return typeof res === "string" ? res : null;
  }
  return "exports";
}

export function ObjectList({ type, search, onOpen }: Props) {
  const { push } = useToast();
  const [items, setItems] = useState<ListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: "name", dir: 1 });
  const [exporting, setExporting] = useState(false);
  const [documenting, setDocumenting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSelected(new Set());
    try {
      const res = await api.listObjects(type.id);
      setItems(res.items);
    } catch (e) {
      setError(e instanceof BackendError ? e.message : (e as Error).message);
    } finally {
      setLoading(false);
    }
  }, [type.id]);

  useEffect(() => {
    load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = q
      ? items.filter((i) => `${i.name} ${i.description ?? ""} ${i.id} ${i.odataType ?? ""}`.toLowerCase().includes(q))
      : items;
    const sorted = [...list].sort((a, b) => {
      const av = (a[sort.key] ?? "").toString().toLowerCase();
      const bv = (b[sort.key] ?? "").toString().toLowerCase();
      return av < bv ? -sort.dir : av > bv ? sort.dir : 0;
    });
    return sorted;
  }, [items, search, sort]);

  const toggleSort = (key: SortKey) =>
    setSort((s) => (s.key === key ? { key, dir: s.dir === 1 ? -1 : 1 } : { key, dir: 1 }));

  const allSelected = filtered.length > 0 && filtered.every((i) => selected.has(i.id));
  const toggleAll = () =>
    setSelected(allSelected ? new Set() : new Set(filtered.map((i) => i.id)));
  const toggleOne = (id: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const runExport = useCallback(
    async (ids: string[] | null) => {
      const dir = await pickDirectory();
      if (dir === null) return;
      setExporting(true);
      try {
        const res = await api.export(type.id, ids, dir);
        push({
          kind: "success",
          title: `Exported ${res.files.length} object${res.files.length === 1 ? "" : "s"}`,
          description: res.directory,
        });
      } catch (e) {
        push({ kind: "error", title: "Export failed", description: (e as Error).message });
      } finally {
        setExporting(false);
      }
    },
    [type.id, push]
  );

  const runDocument = useCallback(
    async (ids: string[] | null, format: DocFormat) => {
      const dir = await pickDirectory();
      if (dir === null) return;
      setDocumenting(true);
      try {
        const res = await api.exportDocumentation(type.id, ids, dir, format);
        push({
          kind: "success",
          title: `Documented ${res.files.length} object${res.files.length === 1 ? "" : "s"} (${format})`,
          description: res.directory,
        });
      } catch (e) {
        push({ kind: "error", title: "Documentation failed", description: (e as Error).message });
      } finally {
        setDocumenting(false);
      }
    },
    [type.id, push]
  );

  const Icon = groupIcon(type.group);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-3 border-b border-border px-6 py-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Icon className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold leading-tight">{type.title}</h2>
          <p className="font-mono text-xs text-muted-foreground">{type.api}</p>
        </div>
        <div className="ml-auto flex items-center gap-2">
          {selected.size > 0 && (
            <Button variant="outline" size="sm" disabled={exporting} onClick={() => runExport([...selected])}>
              {exporting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
              Export selected ({selected.size})
            </Button>
          )}
          <Button variant="secondary" size="sm" disabled={exporting || loading || filtered.length === 0} onClick={() => runExport(null)}>
            {exporting ? <Loader2 className="h-4 w-4 animate-spin" /> : <FolderOpen className="h-4 w-4" />}
            Export all
          </Button>
          <Button
            variant="secondary"
            size="sm"
            disabled={documenting || loading || filtered.length === 0}
            onClick={() => runDocument(selected.size > 0 ? [...selected] : null, "markdown")}
            title="Export Markdown documentation for selected objects, or all if none selected"
          >
            {documenting ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
            {selected.size > 0 ? `Document (${selected.size})` : "Document all"}
          </Button>
          <Button variant="ghost" size="icon" onClick={load} title="Refresh">
            <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {loading ? (
          <div className="space-y-2 p-6">
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} className="skeleton h-12 w-full rounded-lg" />
            ))}
          </div>
        ) : error ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
            <AlertCircle className="h-10 w-10 text-destructive" />
            <div>
              <p className="font-medium">Could not load {type.title}</p>
              <p className="mt-1 max-w-md text-sm text-muted-foreground">{error}</p>
            </div>
            <Button variant="outline" size="sm" onClick={load}>
              <RefreshCw className="h-4 w-4" /> Try again
            </Button>
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center text-muted-foreground">
            <Inbox className="h-10 w-10" />
            <p className="font-medium text-foreground">No objects found</p>
            <p className="max-w-sm text-sm">
              {search ? "No results match your search." : `This tenant has no ${type.title.toLowerCase()} yet.`}
            </p>
          </div>
        ) : (
          <table className="w-full text-sm">
            <thead className="sticky top-0 z-10 bg-background/95 backdrop-blur">
              <tr className="border-b border-border text-left text-xs uppercase tracking-wide text-muted-foreground">
                <th className="w-10 px-6 py-3">
                  <Checkbox checked={allSelected} onChange={toggleAll} />
                </th>
                <SortHeader label="Name" active={sort.key === "name"} onClick={() => toggleSort("name")} />
                <SortHeader label="Type" active={sort.key === "odataType"} onClick={() => toggleSort("odataType")} className="hidden lg:table-cell" />
                <SortHeader label="ID" active={sort.key === "id"} onClick={() => toggleSort("id")} className="hidden xl:table-cell" />
              </tr>
            </thead>
            <tbody>
              {filtered.map((item) => (
                <tr
                  key={item.id}
                  onClick={() => onOpen(item)}
                  className="group cursor-pointer border-b border-border/60 transition-colors hover:bg-accent/50"
                >
                  <td className="px-6 py-3" onClick={(e) => e.stopPropagation()}>
                    <Checkbox checked={selected.has(item.id)} onChange={() => toggleOne(item.id)} />
                  </td>
                  <td className="px-2 py-3">
                    <p className="font-medium text-foreground">{item.name}</p>
                    {item.description && <p className="mt-0.5 line-clamp-1 max-w-xl text-xs text-muted-foreground">{item.description}</p>}
                  </td>
                  <td className="hidden px-2 py-3 lg:table-cell">
                    {item.odataType && <Badge variant="muted">{item.odataType}</Badge>}
                  </td>
                  <td className="hidden px-2 py-3 font-mono text-xs text-muted-foreground xl:table-cell">{item.id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {!loading && !error && (
        <div className="flex items-center justify-between border-t border-border px-6 py-2 text-xs text-muted-foreground">
          <span>
            {filtered.length} of {items.length} object{items.length === 1 ? "" : "s"}
          </span>
          {selected.size > 0 && <span>{selected.size} selected</span>}
        </div>
      )}
    </div>
  );
}

function SortHeader({ label, active, onClick, className }: { label: string; active: boolean; onClick: () => void; className?: string }) {
  return (
    <th className={cn("px-2 py-3", className)}>
      <button onClick={onClick} className={cn("inline-flex items-center gap-1 transition-colors hover:text-foreground", active && "text-foreground")}>
        {label}
        <ArrowUpDown className="h-3 w-3" />
      </button>
    </th>
  );
}

function Checkbox({ checked, onChange }: { checked: boolean; onChange: () => void }) {
  return (
    <button
      role="checkbox"
      aria-checked={checked}
      onClick={onChange}
      className={cn(
        "flex h-4 w-4 items-center justify-center rounded border transition-colors",
        checked ? "border-primary bg-primary text-primary-foreground" : "border-border bg-transparent hover:border-primary"
      )}
    >
      {checked && <Check className="h-3 w-3" />}
    </button>
  );
}
