import { useMemo, useState } from "react";
import { ChevronRight, Layers, Search } from "lucide-react";
import type { ObjectType } from "@/api/types";
import { cn } from "@/lib/utils";
import { groupIcon } from "@/lib/icons";
import { Logo } from "@/components/Logo";
import { Input } from "@/components/ui/input";

interface Props {
  catalog: ObjectType[];
  selectedTypeId: string | null;
  onSelect: (type: ObjectType) => void;
  bulkActive: boolean;
  onSelectBulk: () => void;
}

interface Group {
  name: string;
  order: number;
  items: ObjectType[];
}

export function Sidebar({ catalog, selectedTypeId, onSelect, bulkActive, onSelectBulk }: Props) {
  const [filter, setFilter] = useState("");
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const groups = useMemo<Group[]>(() => {
    const map = new Map<string, Group>();
    for (const t of catalog) {
      if (filter && !t.title.toLowerCase().includes(filter.toLowerCase()) && !t.group.toLowerCase().includes(filter.toLowerCase())) {
        continue;
      }
      if (!map.has(t.group)) map.set(t.group, { name: t.group, order: t.groupOrder, items: [] });
      map.get(t.group)!.items.push(t);
    }
    return [...map.values()].sort((a, b) => a.order - b.order || a.name.localeCompare(b.name));
  }, [catalog, filter]);

  return (
    <aside className="flex h-full w-72 flex-col border-r border-border bg-card/40">
      <div className="flex items-center gap-3 px-5 py-[1.15rem]">
        <Logo className="h-9 w-9" />
        <div className="leading-tight">
          <p className="text-sm font-semibold">Intune Manager</p>
          <p className="text-xs text-muted-foreground">Cross-platform</p>
        </div>
      </div>

      <div className="px-3 pb-2">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter object types…"
            className="pl-8"
          />
        </div>
      </div>

      <div className="px-2 pb-1">
        <button
          onClick={onSelectBulk}
          className={cn(
            "flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm font-medium transition-colors",
            bulkActive ? "bg-primary/15 text-primary" : "text-foreground/80 hover:bg-accent hover:text-accent-foreground"
          )}
        >
          <Layers className="h-4 w-4" />
          Bulk operations
        </button>
      </div>

      <nav className="flex-1 space-y-1 overflow-y-auto px-2 pb-6 pt-1">
        {groups.map((group) => {
          const Icon = groupIcon(group.name);
          const isCollapsed = collapsed[group.name];
          return (
            <div key={group.name}>
              <button
                onClick={() => setCollapsed((c) => ({ ...c, [group.name]: !c[group.name] }))}
                className="flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground transition-colors hover:text-foreground"
              >
                <ChevronRight className={cn("h-3.5 w-3.5 transition-transform", !isCollapsed && "rotate-90")} />
                <Icon className="h-3.5 w-3.5" />
                <span>{group.name}</span>
                <span className="ml-auto text-[10px] font-normal">{group.items.length}</span>
              </button>
              {!isCollapsed && (
                <div className="mb-1 ml-4 border-l border-border pl-2">
                  {group.items.map((t) => {
                    const active = t.id === selectedTypeId;
                    return (
                      <button
                        key={t.id}
                        onClick={() => onSelect(t)}
                        className={cn(
                          "flex w-full items-center rounded-md px-3 py-1.5 text-left text-sm transition-colors",
                          active
                            ? "bg-primary/15 font-medium text-primary"
                            : "text-foreground/80 hover:bg-accent hover:text-accent-foreground"
                        )}
                      >
                        <span className="truncate">{t.title}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
        {groups.length === 0 && <p className="px-4 py-6 text-center text-sm text-muted-foreground">No matching object types</p>}
      </nav>
    </aside>
  );
}
