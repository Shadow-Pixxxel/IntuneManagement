import { useCallback, useState } from "react";

/**
 * User-configurable authentication defaults, persisted in `localStorage`.
 *
 * Only non-secret identifiers live here (client id + tenant). Tokens and
 * client secrets never touch the renderer — they stay in the Rust process.
 *
 * These values pre-fill the device-code sign-in fields so a user who brings
 * their own Entra ID app registration does not have to retype the ids on
 * every launch. Leaving them blank falls back to the built-in Microsoft Graph
 * Command Line Tools public client and the `common` tenant.
 */
export interface AuthSettings {
  /** Override for the device-code OAuth client (application) id. */
  clientId: string;
  /** Override for the sign-in tenant (GUID or domain). */
  tenantId: string;
}

const STORAGE_KEY = "intune-auth-settings";

export const EMPTY_SETTINGS: AuthSettings = { clientId: "", tenantId: "" };

export function loadSettings(): AuthSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...EMPTY_SETTINGS };
    const parsed = JSON.parse(raw) as Partial<AuthSettings>;
    return {
      clientId: typeof parsed.clientId === "string" ? parsed.clientId : "",
      tenantId: typeof parsed.tenantId === "string" ? parsed.tenantId : "",
    };
  } catch {
    return { ...EMPTY_SETTINGS };
  }
}

export function saveSettings(settings: AuthSettings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Storage may be unavailable (private mode); ignore — values stay in memory.
  }
}

export function useAuthSettings() {
  const [settings, setSettings] = useState<AuthSettings>(() => loadSettings());

  const update = useCallback((patch: Partial<AuthSettings>) => {
    setSettings((prev) => {
      const next = { ...prev, ...patch };
      saveSettings(next);
      return next;
    });
  }, []);

  return { settings, update };
}
