import { useCallback, useEffect, useState } from "react";
import { Copy, Loader2, ShieldAlert } from "lucide-react";
import { api } from "@/api/client";
import type { CopyBatchResult, CopyResult, ListItem, ObjectType } from "@/api/types";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { JsonView } from "@/components/JsonView";
import { useToast } from "@/components/ui/toast";

interface Props {
  type: ObjectType;
  /** Single-object mode when provided; otherwise bulk pattern mode. */
  item?: ListItem;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const SafetyBanner = () => (
  <div className="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
    <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0" />
    <span>
      This is a <strong>dry run</strong>. It builds the exact create payload (id, version and assignments stripped, renamed) but
      does not create anything. Real creation is gated off by default for safety.
    </span>
  </div>
);

export function CopyDialog({ type, item, open, onOpenChange }: Props) {
  const { push } = useToast();
  const isPattern = !item;
  const [newName, setNewName] = useState("");
  const [pattern, setPattern] = useState("");
  const [template, setTemplate] = useState("{name} - Copy");
  const [busy, setBusy] = useState(false);
  const [single, setSingle] = useState<CopyResult | null>(null);
  const [batch, setBatch] = useState<CopyBatchResult | null>(null);

  useEffect(() => {
    if (open) {
      setNewName(item ? `${item.name} - Copy` : "");
      setPattern("");
      setTemplate("{name} - Copy");
      setSingle(null);
      setBatch(null);
    }
  }, [open, item]);

  const preview = useCallback(async () => {
    setBusy(true);
    try {
      if (item) {
        const res = await api.copyObject(type.id, item.id, newName || null, false);
        setSingle(res);
      } else {
        const res = await api.copyByPattern(type.id, pattern, template || null, false);
        setBatch(res);
        push({ kind: "info", title: `${res.copies.length} object(s) matched "${pattern}"` });
      }
    } catch (e) {
      push({ kind: "error", title: "Copy preview failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  }, [item, type.id, newName, pattern, template, push]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-hidden">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Copy className="h-4 w-4" /> {isPattern ? "Copy by name pattern" : "Copy object"}
          </DialogTitle>
          <DialogDescription>
            {isPattern
              ? `Clone every ${type.title} whose name matches a pattern. Assignments are not copied.`
              : `Clone this ${type.title} under a new name. Assignments are not copied.`}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3 overflow-auto pr-1">
          <SafetyBanner />

          {isPattern ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="space-y-1">
                <span className="text-xs font-medium text-muted-foreground">Name contains</span>
                <Input value={pattern} onChange={(e) => setPattern(e.target.value)} placeholder="e.g. Defender" />
              </label>
              <label className="space-y-1">
                <span className="text-xs font-medium text-muted-foreground">New name template</span>
                <Input value={template} onChange={(e) => setTemplate(e.target.value)} placeholder="{name} - Copy" />
              </label>
            </div>
          ) : (
            <label className="block space-y-1">
              <span className="text-xs font-medium text-muted-foreground">New name</span>
              <Input value={newName} onChange={(e) => setNewName(e.target.value)} />
            </label>
          )}

          <div className="flex justify-end">
            <Button size="sm" onClick={preview} disabled={busy || (isPattern && !pattern.trim())}>
              {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Copy className="h-4 w-4" />} Preview copy
            </Button>
          </div>

          {single && (
            <div className="space-y-2 rounded-lg border border-border bg-secondary/20 p-3">
              <p className="text-sm">
                <span className="text-muted-foreground">Will create:</span> <strong>{single.newName}</strong>
              </p>
              <p className="text-xs text-muted-foreground">
                Payload keys: {Object.keys(single.payload).length} · id stripped: {String(!("id" in single.payload))} · assignments
                stripped: {String(!("assignments" in single.payload))}
              </p>
              <details className="text-xs">
                <summary className="cursor-pointer text-muted-foreground">View create payload</summary>
                <div className="mt-2 max-h-64 overflow-auto">
                  <JsonView value={single.payload} />
                </div>
              </details>
            </div>
          )}

          {batch && (
            <div className="space-y-2 rounded-lg border border-border bg-secondary/20 p-3">
              <p className="text-sm">
                <strong>{batch.copies.length}</strong> object(s) would be cloned:
              </p>
              <ul className="max-h-64 space-y-1 overflow-auto text-xs">
                {batch.copies.map((c) => (
                  <li key={c.sourceId} className="flex flex-col rounded border border-border/60 px-2 py-1">
                    <span className="text-muted-foreground line-through">{c.sourceName}</span>
                    <span className="font-medium">{c.newName}</span>
                  </li>
                ))}
              </ul>
              {batch.copies.length === 0 && <p className="text-xs text-muted-foreground">No objects matched the pattern.</p>}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
