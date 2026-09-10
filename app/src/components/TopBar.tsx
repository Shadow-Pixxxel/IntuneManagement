import { Building2, LogOut, Moon, Search, Sun } from "lucide-react";
import type { AuthStatus } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import type { Theme } from "@/lib/theme";

interface Props {
  status: AuthStatus;
  search: string;
  onSearch: (v: string) => void;
  searchPlaceholder: string;
  theme: Theme;
  onToggleTheme: () => void;
  onSignOut: () => void;
}

export function TopBar({ status, search, onSearch, searchPlaceholder, theme, onToggleTheme, onSignOut }: Props) {
  return (
    <header className="flex h-16 shrink-0 items-center gap-4 border-b border-border bg-background/80 px-6 backdrop-blur">
      <div className="relative w-full max-w-md">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input value={search} onChange={(e) => onSearch(e.target.value)} placeholder={searchPlaceholder} className="h-10 pl-9" />
      </div>

      <div className="ml-auto flex items-center gap-3">
        <div className="flex items-center gap-2 rounded-lg border border-border bg-card/60 px-3 py-1.5">
          <Building2 className="h-4 w-4 text-primary" />
          <div className="leading-tight">
            <p className="text-sm font-medium">{status.orgName ?? "Signed in"}</p>
            <p className="text-[11px] text-muted-foreground">
              {status.mode === "app-only" ? "App-only" : "Interactive"}
            </p>
          </div>
        </div>

        <Badge variant="success" className="hidden gap-1.5 md:flex">
          <span className="h-1.5 w-1.5 rounded-full bg-success" /> Connected
        </Badge>

        <Button variant="ghost" size="icon" onClick={onToggleTheme} title="Toggle theme">
          {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </Button>

        <Button variant="outline" size="sm" onClick={onSignOut} className="gap-2">
          <LogOut className="h-4 w-4" /> Sign out
        </Button>
      </div>
    </header>
  );
}
