import type {
  AuthStatus,
  CompareResult,
  DeviceCodeStart,
  DocExportResult,
  DocFormat,
  DocumentedObject,
  ExportResult,
  ImportResult,
  ListResult,
  ObjectDetail,
  ObjectType,
} from "./types";

/**
 * Unified backend transport.
 *
 * The same React app runs in two hosts:
 *  - Tauri desktop  -> native `invoke(...)` calls into the Rust core.
 *  - Browser (dev)  -> `fetch('/api/...')` to the axum sidecar (Vite proxy).
 *
 * Secrets and tokens never live here; they stay in the Rust process.
 */
export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export class BackendError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
    this.name = "BackendError";
  }
}

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const err = e as { code?: string; message?: string } | string;
    if (typeof err === "string") throw new BackendError("error", err);
    throw new BackendError(err.code ?? "error", err.message ?? "Unknown error");
  }
}

async function httpGet<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`);
  return handle<T>(res);
}

async function httpPost<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  return handle<T>(res);
}

async function handle<T>(res: Response): Promise<T> {
  const text = await res.text();
  const data = text ? JSON.parse(text) : null;
  if (!res.ok && res.status !== 202) {
    const err = data as { code?: string; message?: string } | null;
    throw new BackendError(err?.code ?? "error", err?.message ?? res.statusText);
  }
  if (res.status === 202) {
    // Device-code authorization still pending.
    throw new BackendError("authorization_pending", "authorization pending");
  }
  return data as T;
}

export const api = {
  async status(): Promise<AuthStatus> {
    return isTauri() ? invokeTauri("auth_status") : httpGet("/status");
  },

  async loginAppOnly(input?: { tenantId?: string; appId?: string; appSecret?: string }): Promise<AuthStatus> {
    return isTauri()
      ? invokeTauri("login_app_only", { tenantId: input?.tenantId, appId: input?.appId, appSecret: input?.appSecret })
      : httpPost("/auth/app-only", input ?? {});
  },

  async deviceStart(input?: { tenantId?: string; appId?: string }): Promise<DeviceCodeStart> {
    return isTauri()
      ? invokeTauri("device_start", { tenantId: input?.tenantId, appId: input?.appId })
      : httpPost("/auth/device/start", input ?? {});
  },

  async devicePoll(input: { tenantId: string; appId: string; deviceCode: string }): Promise<AuthStatus> {
    return isTauri()
      ? invokeTauri("device_poll", input)
      : httpPost("/auth/device/poll", input);
  },

  async logout(): Promise<void> {
    if (isTauri()) {
      await invokeTauri("logout");
    } else {
      await httpPost("/auth/logout");
    }
  },

  async catalog(): Promise<ObjectType[]> {
    return isTauri() ? invokeTauri("catalog") : httpGet("/catalog");
  },

  async listObjects(typeId: string, search?: string): Promise<ListResult> {
    if (isTauri()) return invokeTauri("list_objects", { typeId, search: search || null });
    const q = search ? `?search=${encodeURIComponent(search)}` : "";
    return httpGet(`/objects/${encodeURIComponent(typeId)}${q}`);
  },

  async getObject(typeId: string, id: string): Promise<ObjectDetail> {
    return isTauri()
      ? invokeTauri("get_object", { typeId, id })
      : httpGet(`/objects/${encodeURIComponent(typeId)}/${encodeURIComponent(id)}`);
  },

  async export(typeId: string, ids: string[] | null, outDir: string): Promise<ExportResult> {
    return isTauri()
      ? invokeTauri("export", { typeId, ids, outDir })
      : httpPost("/export", { typeId, ids, outDir });
  },

  async importFile(typeId: string, filePath: string, dryRun: boolean): Promise<ImportResult> {
    return isTauri()
      ? invokeTauri("import_file", { typeId, filePath, dryRun })
      : httpPost("/import", { typeId, filePath, dryRun });
  },

  async compare(typeId: string, id: string, filePath: string): Promise<CompareResult> {
    return isTauri()
      ? invokeTauri("compare_to_file", { typeId, id, filePath })
      : httpPost("/compare", { typeId, id, filePath });
  },

  async documentObject(typeId: string, id: string): Promise<DocumentedObject> {
    return isTauri()
      ? invokeTauri("document_object", { typeId, id })
      : httpGet(`/document/${encodeURIComponent(typeId)}/${encodeURIComponent(id)}`);
  },

  async exportDocumentation(
    typeId: string,
    ids: string[] | null,
    outDir: string,
    format: DocFormat,
  ): Promise<DocExportResult> {
    return isTauri()
      ? invokeTauri("export_documentation", { typeId, ids, outDir, format })
      : httpPost("/document/export", { typeId, ids, outDir, format });
  },
};
