export type AuthMode = "app-only" | "device-code";

export interface AuthStatus {
  authenticated: boolean;
  mode?: AuthMode | null;
  tenantId?: string | null;
  appId?: string | null;
  orgName?: string | null;
  orgId?: string | null;
}

export interface DeviceCodeStart {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  interval: number;
  message: string;
  tenantId: string;
  appId: string;
}

export interface ObjectType {
  id: string;
  title: string;
  api: string;
  group: string;
  groupOrder: number;
  nameProperty: string;
  queryList?: string | null;
  expand?: string | null;
  assignments: boolean;
  odataTypeFilter?: string | null;
  icon: string;
}

export interface ListItem {
  id: string;
  name: string;
  description?: string | null;
  odataType?: string | null;
}

export interface ListResult {
  typeId: string;
  nameProperty: string;
  count: number;
  items: ListItem[];
}

export interface ObjectDetail {
  id: string;
  name: string;
  object: Record<string, unknown>;
  assignments?: unknown;
}

export interface ExportedFile {
  id: string;
  name: string;
  path: string;
}

export interface ExportResult {
  typeId: string;
  directory: string;
  files: ExportedFile[];
}

export interface ImportResult {
  typeId: string;
  dryRun: boolean;
  payload: Record<string, unknown>;
  created?: unknown;
  targetApi: string;
}

export type ChangeKind = "added" | "removed" | "changed";

export interface Difference {
  path: string;
  kind: ChangeKind;
  left?: unknown;
  right?: unknown;
}

export interface CompareResult {
  identical: boolean;
  added: number;
  removed: number;
  changed: number;
  differences: Difference[];
}

export interface ApiError {
  code: string;
  message: string;
}

export type DocFormat = "markdown" | "html" | "json";

export type DocRowKind = "setting" | "group";

export interface DocRow {
  name: string;
  value: string;
  level: number;
  kind: DocRowKind;
}

export interface DocSection {
  title: string;
  rows: DocRow[];
}

export interface DocumentedObject {
  typeId: string;
  typeTitle: string;
  objectId: string;
  name: string;
  description?: string | null;
  sections: DocSection[];
}

export interface DocExportedFile {
  id: string;
  name: string;
  path: string;
  format: DocFormat;
}

export interface DocExportResult {
  directory: string;
  files: DocExportedFile[];
}

export interface CopyResult {
  typeId: string;
  sourceId: string;
  sourceName: string;
  newName: string;
  payload: Record<string, unknown>;
  created?: unknown;
  applied: boolean;
  targetApi: string;
}

export interface CopyBatchResult {
  typeId: string;
  pattern: string;
  applied: boolean;
  copies: CopyResult[];
}

export interface BulkExportResult {
  root: string;
  totalFiles: number;
  results: ExportResult[];
}

export interface BulkImportItem {
  typeId: string;
  typeTitle: string;
  file: string;
  name: string;
  payload: Record<string, unknown>;
  created?: unknown;
  error?: string | null;
}

export interface BulkImportResult {
  root: string;
  dryRun: boolean;
  order: string[];
  items: BulkImportItem[];
}

export interface BulkCompareItem {
  typeId: string;
  typeTitle: string;
  file: string;
  name: string;
  matched: boolean;
  identical: boolean;
  added: number;
  removed: number;
  changed: number;
  error?: string | null;
}

export interface BulkCompareResult {
  root: string;
  items: BulkCompareItem[];
}
