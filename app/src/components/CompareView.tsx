import { CheckCircle2, Minus, Pencil, Plus } from "lucide-react";
import type { CompareResult, Difference } from "@/api/types";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

const kindMeta = {
  added: { icon: Plus, label: "Added", cls: "text-success", badge: "success" as const },
  removed: { icon: Minus, label: "Removed", cls: "text-destructive", badge: "destructive" as const },
  changed: { icon: Pencil, label: "Changed", cls: "text-amber-500", badge: "secondary" as const },
};

function preview(v: unknown): string {
  if (v === undefined) return "—";
  const s = typeof v === "string" ? v : JSON.stringify(v);
  return s.length > 160 ? s.slice(0, 160) + "…" : s;
}

export function CompareView({ result }: { result: CompareResult }) {
  if (result.identical) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
        <CheckCircle2 className="h-12 w-12 text-success" />
        <p className="font-medium">Objects are identical</p>
        <p className="text-sm text-muted-foreground">The live Intune object matches the exported file (ignoring ids and timestamps).</p>
      </div>
    );
  }
  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <Badge variant="success">+{result.added} added</Badge>
        <Badge variant="destructive">-{result.removed} removed</Badge>
        <Badge variant="secondary">~{result.changed} changed</Badge>
      </div>
      <div className="space-y-2">
        {result.differences.map((d, i) => (
          <DiffRow key={i} diff={d} />
        ))}
      </div>
      <p className="pt-1 text-xs text-muted-foreground">Left = live Intune object · Right = exported file</p>
    </div>
  );
}

function DiffRow({ diff }: { diff: Difference }) {
  const meta = kindMeta[diff.kind];
  const Icon = meta.icon;
  return (
    <div className="rounded-lg border border-border bg-secondary/20 p-3">
      <div className="flex items-center gap-2">
        <Icon className={cn("h-4 w-4", meta.cls)} />
        <span className="font-mono text-xs">{diff.path}</span>
        <Badge variant={meta.badge} className="ml-auto">
          {meta.label}
        </Badge>
      </div>
      {diff.kind !== "added" && (
        <div className="mt-2 rounded border border-destructive/30 bg-destructive/5 px-2 py-1 font-mono text-xs text-destructive">
          − {preview(diff.left)}
        </div>
      )}
      {diff.kind !== "removed" && (
        <div className="mt-1 rounded border border-success/30 bg-success/5 px-2 py-1 font-mono text-xs text-success">
          + {preview(diff.right)}
        </div>
      )}
    </div>
  );
}
