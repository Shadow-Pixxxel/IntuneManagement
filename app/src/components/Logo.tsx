import { cn } from "@/lib/utils";

/** App wordmark glyph — a hexagon + check, matching the desktop icon. */
export function Logo({ className }: { className?: string }) {
  return (
    <div className={cn("relative flex items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500 to-violet-600 shadow-lg shadow-primary/30", className)}>
      <svg viewBox="0 0 100 100" fill="none" className="h-3/5 w-3/5">
        <polygon
          points="50,12 83,31 83,69 50,88 17,69 17,31"
          stroke="white"
          strokeWidth="6"
          strokeLinejoin="round"
          fill="none"
          opacity="0.95"
        />
        <path d="M34 52 L46 65 L68 37" stroke="white" strokeWidth="8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </div>
  );
}
