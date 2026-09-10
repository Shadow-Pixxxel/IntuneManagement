import { useCallback, useEffect, useState } from "react";
import { Loader2, MousePointerClick } from "lucide-react";
import { api } from "@/api/client";
import type { AuthStatus, ListItem, ObjectType } from "@/api/types";
import { SignIn } from "@/components/SignIn";
import { Sidebar } from "@/components/Sidebar";
import { TopBar } from "@/components/TopBar";
import { ObjectList } from "@/components/ObjectList";
import { ObjectDetail } from "@/components/ObjectDetail";
import { BulkView } from "@/components/BulkView";
import { Logo } from "@/components/Logo";
import { useTheme } from "@/lib/theme";
import { useToast } from "@/components/ui/toast";

export default function App() {
  const { theme, toggle } = useTheme();
  const { push } = useToast();
  const [booting, setBooting] = useState(true);
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [catalog, setCatalog] = useState<ObjectType[]>([]);
  const [selectedType, setSelectedType] = useState<ObjectType | null>(null);
  const [selectedItem, setSelectedItem] = useState<ListItem | null>(null);
  const [search, setSearch] = useState("");
  const [bulk, setBulk] = useState(false);

  useEffect(() => {
    api
      .status()
      .then((s) => setStatus(s))
      .catch(() => setStatus({ authenticated: false }))
      .finally(() => setBooting(false));
  }, []);

  const loadCatalog = useCallback(async () => {
    try {
      setCatalog(await api.catalog());
    } catch (e) {
      push({ kind: "error", title: "Failed to load catalog", description: (e as Error).message });
    }
  }, [push]);

  useEffect(() => {
    if (status?.authenticated) loadCatalog();
  }, [status?.authenticated, loadCatalog]);

  const onSignedIn = useCallback((s: AuthStatus) => {
    setStatus(s);
    push({ kind: "success", title: "Signed in", description: s.orgName ?? undefined });
  }, [push]);

  const onSignOut = useCallback(async () => {
    await api.logout();
    setStatus({ authenticated: false });
    setSelectedType(null);
    setSelectedItem(null);
    setCatalog([]);
  }, []);

  const onSelectType = useCallback((t: ObjectType) => {
    setSelectedType(t);
    setSelectedItem(null);
    setSearch("");
    setBulk(false);
  }, []);

  const onSelectBulk = useCallback(() => {
    setBulk(true);
    setSelectedType(null);
    setSelectedItem(null);
  }, []);

  if (booting) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Logo className="h-12 w-12" />
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (!status?.authenticated) {
    return <SignIn onSignedIn={onSignedIn} />;
  }

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar
        catalog={catalog}
        selectedTypeId={selectedType?.id ?? null}
        onSelect={onSelectType}
        bulkActive={bulk}
        onSelectBulk={onSelectBulk}
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar
          status={status}
          search={search}
          onSearch={setSearch}
          searchPlaceholder={selectedType ? `Search ${selectedType.title.toLowerCase()}…` : "Select an object type to begin"}
          theme={theme}
          onToggleTheme={toggle}
          onSignOut={onSignOut}
        />
        <main className="min-h-0 flex-1">
          {bulk ? (
            <BulkView catalog={catalog} />
          ) : selectedType ? (
            <ObjectList type={selectedType} search={search} onOpen={setSelectedItem} />
          ) : (
            <WelcomeScreen count={catalog.length} />
          )}
        </main>
      </div>

      {selectedType && selectedItem && (
        <ObjectDetail type={selectedType} item={selectedItem} onClose={() => setSelectedItem(null)} />
      )}
    </div>
  );
}

function WelcomeScreen({ count }: { count: number }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
      <Logo className="h-16 w-16" />
      <div>
        <h2 className="text-2xl font-semibold tracking-tight">Welcome to Intune Manager</h2>
        <p className="mt-2 max-w-md text-muted-foreground">
          Browse, export and compare your Microsoft Intune configuration across{" "}
          <span className="font-medium text-foreground">{count}</span> object types. Pick a type from the sidebar to get
          started.
        </p>
      </div>
      <div className="mt-2 flex items-center gap-2 rounded-lg border border-border bg-card/60 px-4 py-2 text-sm text-muted-foreground">
        <MousePointerClick className="h-4 w-4" /> Select an object type on the left
      </div>
    </div>
  );
}
