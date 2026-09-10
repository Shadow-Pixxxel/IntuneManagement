import { useCallback, useEffect, useRef, useState } from "react";
import { Copy, ExternalLink, KeyRound, Loader2, ShieldCheck, Smartphone } from "lucide-react";
import { api, BackendError, isTauri } from "@/api/client";
import type { AuthStatus, DeviceCodeStart } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import { Logo } from "@/components/Logo";

interface Props {
  onSignedIn: (status: AuthStatus) => void;
}

async function openExternal(url: string) {
  if (isTauri()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}

export function SignIn({ onSignedIn }: Props) {
  const { push } = useToast();
  const [busy, setBusy] = useState(false);
  const [device, setDevice] = useState<DeviceCodeStart | null>(null);
  const [tenantId, setTenantId] = useState("");
  const [appId, setAppId] = useState("");
  const [appSecret, setAppSecret] = useState("");
  const pollRef = useRef<number | null>(null);

  const stopPolling = () => {
    if (pollRef.current) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
  };
  useEffect(() => stopPolling, []);

  const startDevice = useCallback(async () => {
    setBusy(true);
    try {
      const dc = await api.deviceStart({ tenantId: tenantId || undefined, appId: appId || undefined });
      setDevice(dc);
      await openExternal(dc.verificationUri);
      pollRef.current = window.setInterval(async () => {
        try {
          const status = await api.devicePoll({ tenantId: dc.tenantId, appId: dc.appId, deviceCode: dc.deviceCode });
          stopPolling();
          onSignedIn(status);
        } catch (e) {
          if (!(e instanceof BackendError) || e.code !== "authorization_pending") {
            stopPolling();
            setDevice(null);
            push({ kind: "error", title: "Sign-in failed", description: (e as Error).message });
          }
        }
      }, Math.max(dc.interval, 3) * 1000);
    } catch (e) {
      push({ kind: "error", title: "Could not start device sign-in", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  }, [tenantId, appId, onSignedIn, push]);

  const appOnly = useCallback(async () => {
    setBusy(true);
    try {
      const status = await api.loginAppOnly({
        tenantId: tenantId || undefined,
        appId: appId || undefined,
        appSecret: appSecret || undefined,
      });
      onSignedIn(status);
    } catch (e) {
      push({ kind: "error", title: "Sign-in failed", description: (e as Error).message });
    } finally {
      setBusy(false);
    }
  }, [tenantId, appId, appSecret, onSignedIn, push]);

  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-background p-6">
      <div className="pointer-events-none absolute -top-40 left-1/2 h-[38rem] w-[38rem] -translate-x-1/2 rounded-full bg-primary/20 blur-3xl" />
      <div className="pointer-events-none absolute bottom-[-10rem] right-[-6rem] h-[28rem] w-[28rem] rounded-full bg-primary/10 blur-3xl" />

      <div className="relative z-10 w-full max-w-md animate-fade-in">
        <div className="mb-8 flex flex-col items-center text-center">
          <Logo className="h-14 w-14" />
          <h1 className="mt-4 text-2xl font-semibold tracking-tight">Intune Manager</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            A beautiful, cross-platform console for Microsoft Intune
          </p>
        </div>

        <div className="rounded-2xl border border-border bg-card/80 p-6 shadow-xl backdrop-blur">
          <Tabs defaultValue="device">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="device" className="gap-2">
                <Smartphone className="h-4 w-4" /> Device code
              </TabsTrigger>
              <TabsTrigger value="app" className="gap-2">
                <KeyRound className="h-4 w-4" /> App credentials
              </TabsTrigger>
            </TabsList>

            <TabsContent value="device">
              {device ? (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    Enter this code at{" "}
                    <button className="font-medium text-primary hover:underline" onClick={() => openExternal(device.verificationUri)}>
                      {device.verificationUri.replace(/^https?:\/\//, "")}
                    </button>
                  </p>
                  <div className="flex items-center justify-between rounded-lg border border-border bg-secondary/50 px-4 py-3">
                    <span className="font-mono text-2xl font-semibold tracking-[0.3em]">{device.userCode}</span>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        navigator.clipboard.writeText(device.userCode);
                        push({ kind: "success", title: "Code copied" });
                      }}
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" /> Waiting for you to approve sign-in…
                  </div>
                  <div className="flex gap-2">
                    <Button variant="outline" className="flex-1" onClick={() => openExternal(device.verificationUri)}>
                      <ExternalLink className="h-4 w-4" /> Open portal
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={() => {
                        stopPolling();
                        setDevice(null);
                      }}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    Sign in interactively with your Microsoft account. Best for Linux, Wayland and Omarchy — no embedded
                    browser required.
                  </p>
                  <Input placeholder="Tenant ID (optional — defaults to 'common')" value={tenantId} onChange={(e) => setTenantId(e.target.value)} />
                  <Input placeholder="Application (client) ID (optional)" value={appId} onChange={(e) => setAppId(e.target.value)} />
                  <Button className="w-full" disabled={busy} onClick={startDevice}>
                    {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Smartphone className="h-4 w-4" />}
                    Sign in with device code
                  </Button>
                </div>
              )}
            </TabsContent>

            <TabsContent value="app">
              <div className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  App-only sign in with client credentials. Leave fields blank to use the{" "}
                  <code className="rounded bg-muted px-1 py-0.5 text-xs">tenant_id</code>,{" "}
                  <code className="rounded bg-muted px-1 py-0.5 text-xs">app_id</code> and{" "}
                  <code className="rounded bg-muted px-1 py-0.5 text-xs">app_secret</code> environment variables.
                </p>
                <Input placeholder="Tenant ID" value={tenantId} onChange={(e) => setTenantId(e.target.value)} />
                <Input placeholder="Application (client) ID" value={appId} onChange={(e) => setAppId(e.target.value)} />
                <Input type="password" placeholder="Client secret" value={appSecret} onChange={(e) => setAppSecret(e.target.value)} />
                <Button className="w-full" disabled={busy} onClick={appOnly}>
                  {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <ShieldCheck className="h-4 w-4" />}
                  Sign in with app credentials
                </Button>
              </div>
            </TabsContent>
          </Tabs>
        </div>
        <p className="mt-6 text-center text-xs text-muted-foreground">
          Connects to Microsoft Graph <span className="font-mono">beta</span>. Tokens stay on this device.
        </p>
      </div>
    </div>
  );
}
