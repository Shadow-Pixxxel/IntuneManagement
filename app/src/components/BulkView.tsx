import { useMemo, useState } from "react";
import { CheckCircle2, Download, FolderInput, GitCompare, Layers, Loader2, ShieldAlert, XCircle } from "lucide-react";
import { api, isTauri } from "@/api/client";
import type { BulkCompareResult, BulkExportResult, BulkImportResult, ObjectType } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

interface Props {
  catalog: ObjectType[];
}

async function pickDir(title: string): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ directory: true, multiple: false, title });
    return typeof res === "string" ? res : null;
  }
  return window.prompt(title, "exports");
}

export function BulkView({ catalog }: Props) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-3 border-b border-border px-6 py-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Layers className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold leading-tight">Bulk operations</h2>
          <p className="text-xs text-muted-foreground">Export, import and compare many object types at once</p>
        </div>
      </div>
      <div className="flex-1 overflow-auto p-6">
        <Tabs defaultValue="export">
          <TabsList>
            <TabsTrigger value="export" className="gap-1.5">
              <Download className="h-3.5 w-3.5" /> Export
            </TabsTrigger>
            <TabsTrigger value="import" className="gap-1.5">
              <FolderInput className="h-3.5 w-3.5" /> Import
            </TabsTrigger>
            <TabsTrigger value="compare" className="gap-1.5">
              <GitCompare className="h-3.5 w-3.5" /> Compare
            </TabsTrigger>
          </TabsList>
          <TabsContent value="export" className="pt-4">
            <BulkExport catalog={catalog} />
          </TabsContent>
          <TabsContent value="import" className="pt-4">
            <BulkImport />
          </TabsContent>
          <TabsContent value="compare" className="pt-4">
            <BulkCompare />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}

function BulkExport({ catalog }: { catalog: ObjectType[] }) {
  const { push } = useToast();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<BulkExportResult | null>(null);

  const groups = useMemo(() => {
    const map = new Map<string, ObjectType[]>();
    for (const t of catalog) {
      if (!map.has(t.group)) map.set(t.group, []);
      map.get(t.group)!.push(t);
    }
    return [...map.entries()];
  }, [catalog]);

  const toggle = (id: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const run = async () => {
    const dir = await pickDir("Choose export folder");
    if (dir === null) return;
    setBusy(true);
    try {
      const res = await api.bulkExport([...selected], dir);
      setResult(res);
      push({ kind: "success", title: `Exported ${res.totalFiles} file(s)`, description: res.root });
    } catch (e) {
      push({ kind: "error", title: "Bulk export failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Select the object types to export. Each type is written to its own subfolder under the chosen directory.
      </p>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="outline" onClick={() => setSelected(new Set(catalog.map((t) => t.id)))}>
          Select all
        </Button>
        <Button size="sm" variant="outline" onClick={() => setSelected(new Set())}>
          Clear
        </Button>
        <Button size="sm" disabled={busy || selected.size === 0} onClick={run}>
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />} Export {selected.size} type(s)
        </Button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {groups.map(([group, types]) => (
          <div key={group} className="rounded-lg border border-border p-3">
            <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{group}</p>
            <div className="space-y-1">
              {types.map((t) => (
                <label key={t.id} className="flex cursor-pointer items-center gap-2 text-sm">
                  <input type="checkbox" checked={selected.has(t.id)} onChange={() => toggle(t.id)} className="accent-primary" />
                  <span className="truncate">{t.title}</span>
                </label>
              ))}
            </div>
          </div>
        ))}
      </div>
      {result && (
        <div className="rounded-lg border border-border bg-secondary/20 p-3 text-sm">
          <p className="mb-2 font-medium">
            {result.totalFiles} file(s) exported to <span className="font-mono text-xs">{result.root}</span>
          </p>
          <ul className="space-y-1 text-xs">
            {result.results.map((r) => (
              <li key={r.typeId} className="flex justify-between">
                <span className="text-muted-foreground">{r.directory}</span>
                <Badge variant="muted">{r.files.length}</Badge>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function BulkImport() {
  const { push } = useToast();
  const [root, setRoot] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<BulkImportResult | null>(null);

  const run = async () => {
    let dir = root;
    if (!dir) {
      const picked = await pickDir("Choose exported folder tree");
      if (picked === null) return;
      dir = picked;
      setRoot(picked);
    }
    setBusy(true);
    try {
      const res = await api.bulkImport(dir, true);
      setResult(res);
      push({ kind: "info", title: `Planned ${res.items.length} create(s) (dry run)`, description: res.root });
    } catch (e) {
      push({ kind: "error", title: "Bulk import failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
        <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0" />
        <span>
          Dry run only. Files are read, cleaned (id/version/assignments stripped) and ordered by dependency, but nothing is
          created. Real creation is gated off by default for safety.
        </span>
      </div>
      <div className="flex gap-2">
        <Input value={root} onChange={(e) => setRoot(e.target.value)} placeholder="Path to exported folder tree" />
        <Button disabled={busy} onClick={run}>
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <FolderInput className="h-4 w-4" />} Preview import
        </Button>
      </div>
      {result && (
        <div className="space-y-3">
          <div className="rounded-lg border border-border bg-secondary/20 p-3 text-sm">
            <p className="text-xs text-muted-foreground">Dependency import order</p>
            <div className="mt-1 flex flex-wrap gap-1">
              {result.order.map((id, i) => (
                <Badge key={id} variant="secondary">
                  {i + 1}. {id}
                </Badge>
              ))}
            </div>
          </div>
          <ImportTable result={result} />
        </div>
      )}
    </div>
  );
}

function ImportTable({ result }: { result: BulkImportResult }) {
  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <table className="w-full text-sm">
        <thead className="bg-secondary/30 text-left text-xs uppercase tracking-wide text-muted-foreground">
          <tr>
            <th className="px-3 py-2">Type</th>
            <th className="px-3 py-2">Name</th>
            <th className="px-3 py-2">Status</th>
          </tr>
        </thead>
        <tbody>
          {result.items.map((i, idx) => (
            <tr key={idx} className="border-t border-border/60">
              <td className="px-3 py-2 text-muted-foreground">{i.typeTitle}</td>
              <td className="px-3 py-2 font-medium">{i.name}</td>
              <td className="px-3 py-2">
                {i.error ? (
                  <Badge variant="destructive">{i.error.slice(0, 40)}</Badge>
                ) : (
                  <Badge variant="muted">would create</Badge>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function BulkCompare() {
  const { push } = useToast();
  const [root, setRoot] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<BulkCompareResult | null>(null);

  const run = async () => {
    let dir = root;
    if (!dir) {
      const picked = await pickDir("Choose exported folder tree");
      if (picked === null) return;
      dir = picked;
      setRoot(picked);
    }
    setBusy(true);
    try {
      const res = await api.bulkCompare(dir);
      setResult(res);
      const diffs = res.items.filter((i) => i.matched && !i.identical).length;
      push({ kind: diffs ? "info" : "success", title: `Compared ${res.items.length} file(s)`, description: `${diffs} with differences` });
    } catch (e) {
      push({ kind: "error", title: "Bulk compare failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  };

  const summary = useMemo(() => {
    if (!result) return null;
    const matched = result.items.filter((i) => i.matched);
    const identical = matched.filter((i) => i.identical).length;
    const different = matched.length - identical;
    const unmatched = result.items.length - matched.length;
    return { total: result.items.length, identical, different, unmatched };
  }, [result]);

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Compare an exported folder tree against the live tenant. Each file is matched to a live object by name (read-only).
      </p>
      <div className="flex gap-2">
        <Input value={root} onChange={(e) => setRoot(e.target.value)} placeholder="Path to exported folder tree" />
        <Button disabled={busy} onClick={run}>
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <GitCompare className="h-4 w-4" />} Compare
        </Button>
      </div>
      {summary && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="Compared" value={summary.total} />
          <Stat label="Identical" value={summary.identical} tone="success" />
          <Stat label="Different" value={summary.different} tone={summary.different ? "warn" : undefined} />
          <Stat label="Unmatched" value={summary.unmatched} tone={summary.unmatched ? "warn" : undefined} />
        </div>
      )}
      {result && (
        <div className="overflow-hidden rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead className="bg-secondary/30 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2">Type</th>
                <th className="px-3 py-2">Name</th>
                <th className="px-3 py-2">Result</th>
              </tr>
            </thead>
            <tbody>
              {result.items.map((i, idx) => (
                <tr key={idx} className="border-t border-border/60">
                  <td className="px-3 py-2 text-muted-foreground">{i.typeTitle}</td>
                  <td className="px-3 py-2 font-medium">{i.name}</td>
                  <td className="px-3 py-2">
                    {!i.matched ? (
                      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                        <XCircle className="h-3.5 w-3.5" /> not in tenant
                      </span>
                    ) : i.identical ? (
                      <span className="inline-flex items-center gap-1 text-xs text-emerald-400">
                        <CheckCircle2 className="h-3.5 w-3.5" /> identical
                      </span>
                    ) : (
                      <span className="text-xs text-amber-400">
                        +{i.added} −{i.removed} ~{i.changed}
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function Stat({ label, value, tone }: { label: string; value: number; tone?: "success" | "warn" }) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-secondary/20 px-3 py-2",
        tone === "success" && "border-emerald-500/30",
        tone === "warn" && "border-amber-500/30"
      )}
    >
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className={cn("text-xl font-semibold", tone === "success" && "text-emerald-400", tone === "warn" && "text-amber-400")}>
        {value}
      </p>
    </div>
  );
}
