import { useMemo } from "react";
import { cn } from "@/lib/utils";

/** Lightweight, dependency-free JSON syntax highlighter. */
export function JsonView({ value, className }: { value: unknown; className?: string }) {
  const nodes = useMemo(() => tokenize(JSON.stringify(value, null, 2) ?? "null"), [value]);
  return (
    <pre className={cn("overflow-auto rounded-lg border border-border bg-secondary/30 p-4 text-xs leading-relaxed", className)}>
      <code className="font-mono">{nodes}</code>
    </pre>
  );
}

const COLORS: Record<string, string> = {
  key: "text-primary",
  string: "text-emerald-500 dark:text-emerald-400",
  number: "text-amber-600 dark:text-amber-400",
  boolean: "text-violet-500 dark:text-violet-400",
  null: "text-muted-foreground",
  punct: "text-muted-foreground",
};

function tokenize(json: string) {
  const regex = /("(?:\\.|[^"\\])*")(\s*:)?|\b(true|false)\b|\bnull\b|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g;
  const out: React.ReactNode[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = regex.exec(json)) !== null) {
    if (m.index > last) out.push(json.slice(last, m.index));
    const [full, str, colon, bool, num] = m;
    if (str !== undefined) {
      const cls = colon ? COLORS.key : COLORS.string;
      out.push(
        <span key={i++} className={cls}>
          {str}
        </span>
      );
      if (colon) out.push(<span key={i++} className={COLORS.punct}>{colon}</span>);
    } else if (bool !== undefined) {
      out.push(<span key={i++} className={COLORS.boolean}>{bool}</span>);
    } else if (num !== undefined) {
      out.push(<span key={i++} className={COLORS.number}>{num}</span>);
    } else if (full === "null") {
      out.push(<span key={i++} className={COLORS.null}>null</span>);
    }
    last = regex.lastIndex;
  }
  if (last < json.length) out.push(json.slice(last));
  return out;
}
