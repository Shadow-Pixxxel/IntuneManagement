import { useCallback, useEffect, useState } from "react";
import { AlertCircle, FileText, Loader2 } from "lucide-react";
import { api, BackendError, isTauri } from "@/api/client";
import type { DocFormat, DocumentedObject, ObjectType } from "@/api/types";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

interface Props {
  type: ObjectType;
  id: string;
}

const FORMATS: { key: DocFormat; label: string }[] = [
  { key: "markdown", label: "Markdown" },
  { key: "html", label: "HTML" },
  { key: "json", label: "JSON" },
];

async function pickDir(): Promise<string | null> {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const res = await open({ directory: true, multiple: false, title: "Choose documentation folder" });
    return typeof res === "string" ? res : null;
  }
  return "docs";
}

export function DocumentView({ type, id }: Props) {
  const { push } = useToast();
  const [doc, setDoc] = useState<DocumentedObject | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<DocFormat | null>(null);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    api
      .documentObject(type.id, id)
      .then((d) => active && setDoc(d))
      .catch((e) => active && setError(e instanceof BackendError ? e.message : (e as Error).message))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, [type.id, id]);

  const exportDoc = useCallback(
    async (format: DocFormat) => {
      const dir = await pickDir();
      if (dir === null) return;
      setBusy(format);
      try {
        const res = await api.exportDocumentation(type.id, [id], dir, format);
        push({ kind: "success", title: `Documentation exported (${format})`, description: res.files[0]?.path ?? res.directory });
      } catch (e) {
        push({ kind: "error", title: "Documentation export failed", description: (e as Error).message });
      } finally {
        setBusy(null);
      }
    },
    [type.id, id, push],
  );

  if (loading) {
    return (
      <div className="space-y-3">
        {Array.from({ length: 8 }).map((_, i) => (
          <div key={i} className="skeleton h-7 w-full rounded" />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
        <AlertCircle className="h-10 w-10 text-destructive" />
        <p className="max-w-md text-sm text-muted-foreground">{error}</p>
      </div>
    );
  }

  if (!doc) return null;

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-secondary/20 px-3 py-2">
        <FileText className="h-4 w-4 text-primary" />
        <span className="text-sm text-muted-foreground">Export documentation as</span>
        {FORMATS.map((f) => (
          <Button key={f.key} variant="outline" size="sm" disabled={busy !== null} onClick={() => exportDoc(f.key)}>
            {busy === f.key ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null} {f.label}
          </Button>
        ))}
      </div>

      {doc.description && <p className="text-sm text-muted-foreground">{doc.description}</p>}

      {doc.sections
        .filter((s) => s.rows.length > 0)
        .map((section) => (
          <section key={section.title} className="overflow-hidden rounded-lg border border-border">
            <h3 className="border-b border-border bg-secondary/30 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              {section.title}
            </h3>
            <table className="w-full text-sm">
              <tbody>
                {section.rows.map((row, i) => (
                  <tr key={i} className={cn("border-b border-border/60 last:border-0", row.kind === "group" && "bg-secondary/20")}>
                    <td
                      className={cn("py-2 pr-4 align-top", row.kind === "group" ? "font-semibold" : "text-muted-foreground")}
                      style={{ paddingLeft: `${16 + row.level * 18}px`, width: "50%" }}
                    >
                      {row.name}
                    </td>
                    <td className="whitespace-pre-wrap break-words py-2 pr-4 align-top font-medium">{row.value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ))}
    </div>
  );
}
