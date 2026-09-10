import { useCallback, useEffect, useState } from "react";
import { AlertCircle, Copy, Download, FileText, GitCompare, Loader2, Users, X } from "lucide-react";
import { api, BackendError, isTauri } from "@/api/client";
import type { CompareResult, ListItem, ObjectDetail as Detail, ObjectType } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { JsonView } from "@/components/JsonView";
import { CompareView } from "@/components/CompareView";
import { DocumentView } from "@/components/DocumentView";
import { CopyDialog } from "@/components/CopyDialog";
import { useToast } from "@/components/ui/toast";
import { cn, formatKey } from "@/lib/utils";

interface Props {
  type: ObjectType;
  item: ListItem;
  onClose: () => void;
}

async function pickJsonFile(): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }], title: "Choose a file to compare" });
    return typeof res === "string" ? res : null;
  }
  return window.prompt("Path to exported JSON file to compare against:");
}

async function pickDir(): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ directory: true, multiple: false, title: "Choose export folder" });
    return typeof res === "string" ? res : null;
  }
  return "exports";
}

export function ObjectDetail({ type, item, onClose }: Props) {
  const { push } = useToast();
  const [detail, setDetail] = useState<Detail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [compare, setCompare] = useState<CompareResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [copyOpen, setCopyOpen] = useState(false);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    setCompare(null);
    api
      .getObject(type.id, item.id)
      .then((d) => active && setDetail(d))
      .catch((e) => active && setError(e instanceof BackendError ? e.message : (e as Error).message))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, [type.id, item.id]);

  const exportOne = useCallback(async () => {
    const dir = await pickDir();
    if (dir === null) return;
    setBusy(true);
    try {
      const res = await api.export(type.id, [item.id], dir);
      push({ kind: "success", title: "Exported object", description: res.files[0]?.path ?? res.directory });
    } catch (e) {
      push({ kind: "error", title: "Export failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  }, [type.id, item.id, push]);

  const runCompare = useCallback(async () => {
    const file = await pickJsonFile();
    if (!file) return;
    setBusy(true);
    try {
      const res = await api.compare(type.id, item.id, file);
      setCompare(res);
      push({
        kind: res.identical ? "success" : "info",
        title: res.identical ? "No differences" : `${res.added + res.removed + res.changed} difference(s)`,
      });
    } catch (e) {
      push({ kind: "error", title: "Compare failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  }, [type.id, item.id, push]);

  const assignments = Array.isArray(detail?.assignments) ? (detail!.assignments as Record<string, unknown>[]) : [];

  return (
    <div className="fixed inset-0 z-40 flex justify-end">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm animate-fade-in" onClick={onClose} />
      <div className="relative z-10 flex h-full w-full max-w-3xl flex-col border-l border-border bg-card shadow-2xl animate-fade-in">
        <div className="flex items-start gap-3 border-b border-border p-5">
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{type.title}</p>
            <h2 className="truncate text-xl font-semibold">{item.name}</h2>
            <p className="mt-1 font-mono text-xs text-muted-foreground">{item.id}</p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" disabled={busy || loading} onClick={exportOne}>
              {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />} Export
            </Button>
            <Button variant="outline" size="sm" disabled={busy || loading} onClick={runCompare}>
              <GitCompare className="h-4 w-4" /> Compare
            </Button>
            <Button variant="outline" size="sm" disabled={busy || loading} onClick={() => setCopyOpen(true)}>
              <Copy className="h-4 w-4" /> Copy
            </Button>
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-hidden p-5">
          {loading ? (
            <div className="space-y-3">
              {Array.from({ length: 10 }).map((_, i) => (
                <div key={i} className="skeleton h-8 w-full rounded" />
              ))}
            </div>
          ) : error ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
              <AlertCircle className="h-10 w-10 text-destructive" />
              <p className="max-w-md text-sm text-muted-foreground">{error}</p>
            </div>
          ) : detail ? (
            <Tabs defaultValue="overview" className="flex h-full flex-col">
              <TabsList>
                <TabsTrigger value="overview">Overview</TabsTrigger>
                <TabsTrigger value="documentation" className="gap-1.5">
                  <FileText className="h-3.5 w-3.5" /> Documentation
                </TabsTrigger>
                <TabsTrigger value="json">JSON</TabsTrigger>
                {type.assignments && (
                  <TabsTrigger value="assignments" className="gap-1.5">
                    <Users className="h-3.5 w-3.5" /> Assignments
                    {assignments.length > 0 && <span className="rounded bg-primary/15 px-1.5 text-[10px] text-primary">{assignments.length}</span>}
                  </TabsTrigger>
                )}
                {compare && <TabsTrigger value="compare">Diff</TabsTrigger>}
              </TabsList>

              <TabsContent value="overview" className="min-h-0 flex-1 overflow-auto">
                <Overview object={detail.object} />
              </TabsContent>
              <TabsContent value="documentation" className="min-h-0 flex-1 overflow-auto">
                <DocumentView type={type} id={item.id} />
              </TabsContent>
              <TabsContent value="json" className="min-h-0 flex-1 overflow-hidden">
                <div className="relative h-full">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute right-2 top-2 z-10"
                    onClick={() => {
                      navigator.clipboard.writeText(JSON.stringify(detail.object, null, 2));
                      push({ kind: "success", title: "JSON copied" });
                    }}
                  >
                    <Copy className="h-3.5 w-3.5" /> Copy
                  </Button>
                  <JsonView value={detail.object} className="h-full" />
                </div>
              </TabsContent>
              {type.assignments && (
                <TabsContent value="assignments" className="min-h-0 flex-1 overflow-auto">
                  <AssignmentsView assignments={assignments} />
                </TabsContent>
              )}
              {compare && (
                <TabsContent value="compare" className="min-h-0 flex-1 overflow-auto">
                  <CompareView result={compare} />
                </TabsContent>
              )}
            </Tabs>
          ) : null}
        </div>
      </div>
      <CopyDialog type={type} item={item} open={copyOpen} onOpenChange={setCopyOpen} />
    </div>
  );
}

function Overview({ object }: { object: Record<string, unknown> }) {
  const entries = Object.entries(object).filter(([k]) => !k.startsWith("@odata"));
  const scalars = entries.filter(([, v]) => v === null || typeof v !== "object");
  const complex = entries.filter(([, v]) => v !== null && typeof v === "object");

  return (
    <div className="space-y-6">
      <dl className="grid grid-cols-1 gap-x-6 gap-y-3 sm:grid-cols-2">
        {scalars.map(([k, v]) => (
          <div key={k} className="min-w-0 rounded-lg border border-border bg-secondary/20 px-3 py-2">
            <dt className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{formatKey(k)}</dt>
            <dd className="mt-0.5 break-words text-sm">{renderScalar(v)}</dd>
          </div>
        ))}
      </dl>
      {complex.length > 0 && (
        <div className="space-y-3">
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Nested properties</p>
          {complex.map(([k, v]) => (
            <details key={k} className="rounded-lg border border-border bg-secondary/20">
              <summary className="cursor-pointer px-3 py-2 text-sm font-medium">
                {formatKey(k)}{" "}
                <span className="text-xs text-muted-foreground">
                  {Array.isArray(v) ? `${v.length} item${v.length === 1 ? "" : "s"}` : "object"}
                </span>
              </summary>
              <div className="px-3 pb-3">
                <JsonView value={v} />
              </div>
            </details>
          ))}
        </div>
      )}
    </div>
  );
}

function renderScalar(v: unknown) {
  if (v === null) return <span className="text-muted-foreground">null</span>;
  if (typeof v === "boolean") return <Badge variant={v ? "success" : "muted"}>{String(v)}</Badge>;
  const s = String(v);
  if (/^\d{4}-\d{2}-\d{2}T/.test(s)) return <span title={s}>{new Date(s).toLocaleString()}</span>;
  return <span>{s || <span className="text-muted-foreground">—</span>}</span>;
}

function AssignmentsView({ assignments }: { assignments: Record<string, unknown>[] }) {
  if (assignments.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 py-16 text-center text-muted-foreground">
        <Users className="h-8 w-8" />
        <p>This object is not assigned to any groups.</p>
      </div>
    );
  }
  return (
    <div className="space-y-2">
      {assignments.map((a, i) => {
        const target = (a.target ?? {}) as Record<string, unknown>;
        const targetType = String(target["@odata.type"] ?? "").replace("#microsoft.graph.", "");
        const groupId = target["groupId"] as string | undefined;
        return (
          <div key={i} className="flex items-center gap-3 rounded-lg border border-border bg-secondary/20 px-4 py-3">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/10">
              <Users className="h-4 w-4 text-primary" />
            </div>
            <div className="min-w-0">
              <p className="text-sm font-medium">{formatKey(targetType || "Assignment")}</p>
              {groupId && <p className="font-mono text-xs text-muted-foreground">{groupId}</p>}
            </div>
            {"intent" in a && <Badge variant="secondary" className={cn("ml-auto")}>{String(a.intent)}</Badge>}
          </div>
        );
      })}
    </div>
  );
}
