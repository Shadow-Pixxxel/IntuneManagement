//! Cross-platform core for the Intune Manager desktop app.
//!
//! All Microsoft Graph access, authentication and the export/import/compare
//! feature logic live here. The crate is UI-agnostic: it is wrapped by Tauri
//! commands (`src-tauri`) for the desktop app and by an axum HTTP server
//! (`crates/server`) for browser-based development and headless/DevOps use.
//! Tokens and secrets never leave this process.

pub mod auth;
pub mod catalog;
pub mod compare;
pub mod documentation;
pub mod error;
pub mod graph;
pub mod token_store;

use auth::{AuthStatus, DeviceCodeStart, Session};
use catalog::ObjectType;
use documentation::DocumentedObject;
use error::{CoreError, CoreResult, ErrorBody};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

/// Shared application backend. Cheaply cloneable via `Arc` at the call sites.
pub struct Backend {
    http: reqwest::Client,
    session: Mutex<Option<Session>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub odata_type: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResult {
    pub type_id: String,
    pub name_property: String,
    pub count: usize,
    pub items: Vec<ListItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDetail {
    pub id: String,
    pub name: String,
    pub object: Value,
    pub assignments: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFile {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub type_id: String,
    pub directory: String,
    pub files: Vec<ExportedFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub type_id: String,
    pub dry_run: bool,
    /// The cleaned payload that was (or would be) POSTed to Graph.
    pub payload: Value,
    /// The created object when `dry_run` is false, otherwise null.
    pub created: Option<Value>,
    pub target_api: String,
}

/// Output format for exported documentation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocFormat {
    Markdown,
    Html,
    Json,
}

impl DocFormat {
    fn extension(&self) -> &'static str {
        match self {
            DocFormat::Markdown => "md",
            DocFormat::Html => "html",
            DocFormat::Json => "json",
        }
    }

    fn render(&self, doc: &DocumentedObject) -> String {
        match self {
            DocFormat::Markdown => doc.to_markdown(),
            DocFormat::Html => doc.to_html(),
            DocFormat::Json => serde_json::to_string_pretty(&doc.to_json()).unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocExportedFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub format: DocFormat,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocExportResult {
    pub directory: String,
    pub files: Vec<DocExportedFile>,
}

/// Result of copying (cloning) a single object.
///
/// By default this is a dry run: `payload` holds exactly what *would* be
/// created (id, version and assignments stripped, name replaced) and `created`
/// is null. A real create only happens when `applied` is true.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub type_id: String,
    pub source_id: String,
    pub source_name: String,
    pub new_name: String,
    pub payload: Value,
    pub created: Option<Value>,
    pub applied: bool,
    pub target_api: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyBatchResult {
    pub type_id: String,
    pub pattern: String,
    pub applied: bool,
    pub copies: Vec<CopyResult>,
}

/// Result of exporting several object types at once into one folder tree.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkExportResult {
    pub root: String,
    pub total_files: usize,
    pub results: Vec<ExportResult>,
}

/// One planned/applied create in a bulk import.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportItem {
    pub type_id: String,
    pub type_title: String,
    pub file: String,
    pub name: String,
    pub payload: Value,
    pub created: Option<Value>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportResult {
    pub root: String,
    pub dry_run: bool,
    /// Type ids in the order they are imported (dependency order).
    pub order: Vec<String>,
    pub items: Vec<BulkImportItem>,
}

/// One file compared against the live tenant in a bulk compare.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCompareItem {
    pub type_id: String,
    pub type_title: String,
    pub file: String,
    pub name: String,
    pub matched: bool,
    pub identical: bool,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCompareResult {
    pub root: String,
    pub items: Vec<BulkCompareItem>,
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("IntuneManager-CrossPlatform/0.1")
            .build()
            .expect("build http client");
        Backend { http, session: Mutex::new(None) }
    }

    // ---- Auth -------------------------------------------------------------

    pub async fn status(&self) -> AuthStatus {
        match &*self.session.lock().await {
            Some(s) => s.status(),
            None => AuthStatus::signed_out(),
        }
    }

    async fn enrich_org(&self, session: &mut Session) {
        if let Ok(token) = session.token(&self.http).await {
            let url = graph::url("/organization", Some("$select=id,displayName"));
            if let Ok(v) = graph::get(&self.http, &token, &url).await {
                if let Some(org) = v.get("value").and_then(|a| a.as_array()).and_then(|a| a.first()) {
                    session.org_name = org.get("displayName").and_then(|x| x.as_str()).map(String::from);
                    session.org_id = org.get("id").and_then(|x| x.as_str()).map(String::from);
                }
            }
        }
    }

    pub async fn login_app_only(
        &self,
        tenant_id: Option<String>,
        app_id: Option<String>,
        app_secret: Option<String>,
    ) -> CoreResult<AuthStatus> {
        let mut session = auth::login_app_only(&self.http, tenant_id, app_id, app_secret).await?;
        self.enrich_org(&mut session).await;
        let status = session.status();
        *self.session.lock().await = Some(session);
        Ok(status)
    }

    pub async fn device_start(&self, tenant_id: Option<String>, app_id: Option<String>) -> CoreResult<DeviceCodeStart> {
        auth::device_code_start(&self.http, tenant_id, app_id).await
    }

    pub async fn device_poll(&self, tenant_id: &str, app_id: &str, device_code: &str) -> CoreResult<AuthStatus> {
        let mut session = auth::device_code_poll(&self.http, tenant_id, app_id, device_code).await?;
        self.enrich_org(&mut session).await;
        self.persist_session(&session);
        let status = session.status();
        *self.session.lock().await = Some(session);
        Ok(status)
    }

    /// Try to restore a delegated session from the OS keyring at startup.
    /// Best-effort: silently does nothing if no token is stored or the keyring
    /// is unavailable. Returns whether a session was restored.
    pub async fn try_restore(&self) -> bool {
        if self.session.lock().await.is_some() {
            return true;
        }
        let Some(persisted) = token_store::load() else { return false };
        match auth::restore_from_refresh(&self.http, &persisted.tenant_id, &persisted.app_id, &persisted.refresh_token).await {
            Ok(mut session) => {
                self.enrich_org(&mut session).await;
                self.persist_session(&session);
                *self.session.lock().await = Some(session);
                true
            }
            Err(e) => {
                tracing::debug!("session restore failed: {e:?}");
                token_store::clear();
                false
            }
        }
    }

    /// Persist a device-code session's refresh token to the OS keyring, if one
    /// is available. App-only sessions are not persisted (the secret lives in
    /// the environment / caller).
    fn persist_session(&self, session: &Session) {
        if session.mode == auth::AuthMode::DeviceCode {
            if let Some(refresh) = session.refresh_token() {
                token_store::save(&token_store::PersistedAuth {
                    tenant_id: session.tenant_id.clone(),
                    app_id: session.app_id.clone(),
                    refresh_token: refresh.to_string(),
                });
            }
        }
    }

    pub async fn logout(&self) {
        token_store::clear();
        *self.session.lock().await = None;
    }

    async fn token(&self) -> CoreResult<String> {
        let mut guard = self.session.lock().await;
        let s = guard.as_mut().ok_or(CoreError::NotAuthenticated)?;
        s.token(&self.http).await
    }

    // ---- Catalog ----------------------------------------------------------

    pub fn catalog(&self) -> Vec<catalog::ObjectType> {
        catalog::catalog()
    }

    // ---- Browse -----------------------------------------------------------

    pub async fn list_objects(&self, type_id: &str, search: Option<&str>) -> CoreResult<ListResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let token = self.token().await?;
        let url = graph::url(&t.api, t.query_list.as_deref());
        let raw = graph::get_all(&self.http, &token, &url).await?;
        let search_lc = search.map(|s| s.to_lowercase());
        let mut items = Vec::new();
        for obj in raw {
            let ot = obj.get("@odata.type").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            if let Some(filter) = &t.odata_type_filter {
                let any = filter.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).any(|s| ot.contains(&s));
                if !any {
                    continue;
                }
            }
            if let Some(exclude) = &t.odata_type_exclude {
                let excluded = exclude.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).any(|s| ot.contains(&s));
                if excluded {
                    continue;
                }
            }
            let name = display_name(&obj, &t.name_property);
            let description = obj.get("description").and_then(|v| v.as_str()).map(String::from);
            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let odata_type = obj.get("@odata.type").and_then(|v| v.as_str()).map(short_type);
            if let Some(q) = &search_lc {
                let hay = format!("{} {} {}", name, description.clone().unwrap_or_default(), id).to_lowercase();
                if !hay.contains(q) {
                    continue;
                }
            }
            items.push(ListItem { id, name, description, odata_type });
        }
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(ListResult { type_id: t.id.clone(), name_property: t.name_property.clone(), count: items.len(), items })
    }

    // ---- Detail -----------------------------------------------------------

    pub async fn get_object(&self, type_id: &str, id: &str) -> CoreResult<ObjectDetail> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let token = self.token().await?;
        let query = t.expand.as_ref().map(|e| format!("$expand={e}"));
        let url = graph::url(&format!("{}/{}", t.api, id), query.as_deref());
        let mut object = graph::get(&self.http, &token, &url).await?;
        self.apply_special_processing(&t, id, &token, &mut object).await;
        let name = display_name(&object, &t.name_property);
        let assignments = if t.assignments {
            let aurl = graph::url(&format!("{}/{}/assignments", t.api, id), None);
            graph::get(&self.http, &token, &aurl).await.ok().and_then(|v| v.get("value").cloned())
        } else {
            None
        };
        Ok(ObjectDetail { id: id.to_string(), name, object, assignments })
    }

    /// Object-type-specific enrichment that a plain GET does not return:
    /// Endpoint Security intent `settings` and Administrative Template
    /// `definitionValues` (ADMX) are separate Graph collections and are merged
    /// into the object so export and documentation see the full configuration.
    async fn apply_special_processing(&self, t: &ObjectType, id: &str, token: &str, object: &mut Value) {
        match t.id.as_str() {
            "EndpointSecurity" => {
                let url = graph::url(&format!("/deviceManagement/intents/{id}/settings"), None);
                if let Ok(settings) = graph::get_all(&self.http, token, &url).await {
                    if let Some(map) = object.as_object_mut() {
                        map.insert("settings".into(), Value::Array(settings));
                    }
                }
            }
            "AdministrativeTemplates" => {
                let q = "$expand=definition($select=id,displayName,categoryPath,classType,policyType),presentationValues($expand=presentation)";
                let url = graph::url(&format!("/deviceManagement/groupPolicyConfigurations/{id}/definitionValues"), Some(q));
                if let Ok(values) = graph::get_all(&self.http, token, &url).await {
                    if let Some(map) = object.as_object_mut() {
                        map.insert("definitionValues".into(), Value::Array(values));
                    }
                }
            }
            _ => {}
        }
    }

    // ---- Export -----------------------------------------------------------

    pub async fn export(&self, type_id: &str, ids: Option<Vec<String>>, out_dir: &str) -> CoreResult<ExportResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let target_ids = match ids {
            Some(v) if !v.is_empty() => v,
            _ => self.list_objects(type_id, None).await?.items.into_iter().map(|i| i.id).collect(),
        };
        let dir = std::path::Path::new(out_dir).join(&t.title);
        std::fs::create_dir_all(&dir)?;
        let mut files = Vec::new();
        for id in target_ids {
            let detail = self.get_object(type_id, &id).await?;
            let mut export_obj = detail.object.clone();
            if let Some(assignments) = &detail.assignments {
                if let Some(map) = export_obj.as_object_mut() {
                    map.insert("assignments".into(), assignments.clone());
                }
            }
            let filename = format!("{}.json", sanitize(&detail.name));
            let path = dir.join(&filename);
            std::fs::write(&path, serde_json::to_string_pretty(&export_obj).unwrap())?;
            files.push(ExportedFile { id, name: detail.name, path: path.to_string_lossy().to_string() });
        }
        Ok(ExportResult { type_id: t.id.clone(), directory: dir.to_string_lossy().to_string(), files })
    }

    // ---- Import -----------------------------------------------------------

    /// Import (create) an object from an exported JSON file.
    /// Defaults to a dry run: the cleaned payload is returned but nothing is sent to Graph.
    pub async fn import_file(&self, type_id: &str, file_path: &str, dry_run: bool) -> CoreResult<ImportResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let text = std::fs::read_to_string(file_path)?;
        let parsed: Value = serde_json::from_str(&text).map_err(|e| CoreError::Other(format!("parse import file: {e}")))?;
        let payload = clean_for_import(parsed);
        let target_api = graph::url(&t.api, None);
        let created = if dry_run {
            None
        } else {
            self.create_object(&target_api, &payload).await?
        };
        Ok(ImportResult { type_id: t.id.clone(), dry_run, payload, created, target_api })
    }

    // ---- Documentation ----------------------------------------------------

    /// Build a human-readable documentation model for a single object.
    ///
    /// Settings Catalog and Compliance V2 objects are resolved against the
    /// Graph `settingDefinitions` so the output reads like the Intune portal;
    /// all other types fall back to a generic property renderer.
    pub async fn document_object(&self, type_id: &str, id: &str) -> CoreResult<DocumentedObject> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let detail = self.get_object(type_id, id).await?;

        let description = detail
            .object
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let mut sections = Vec::new();

        if documentation::is_settings_catalog(type_id) {
            let token = self.token().await?;
            let url = graph::url(&documentation::settings_endpoint(&t, id), None);
            let settings = graph::get_all(&self.http, &token, &url).await.unwrap_or_default();
            let section = documentation::settings_catalog_section(&settings);
            if section.rows.is_empty() {
                sections.push(documentation::generic_section(&detail.object));
            } else {
                sections.push(section);
            }
        } else if type_id == "EndpointSecurity" {
            // `settings` was merged in by special processing during get_object.
            let settings = detail.object.get("settings").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let section = documentation::endpoint_security_section(&settings);
            if section.rows.is_empty() {
                sections.push(documentation::generic_section(&detail.object));
            } else {
                sections.push(section);
            }
        } else if type_id == "AdministrativeTemplates" {
            let values = detail.object.get("definitionValues").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let section = documentation::admx_section(&values);
            if section.rows.is_empty() {
                sections.push(documentation::generic_section(&detail.object));
            } else {
                sections.push(section);
            }
        } else {
            sections.push(documentation::generic_section(&detail.object));
        }

        if let Some(assignments) = &detail.assignments {
            if let Some(section) = documentation::assignments_section(assignments) {
                sections.push(section);
            }
        }

        sections.push(documentation::metadata_section(&detail.object));

        Ok(DocumentedObject {
            type_id: t.id.clone(),
            type_title: t.title.clone(),
            object_id: id.to_string(),
            name: detail.name,
            description,
            sections,
        })
    }

    /// Document one or more objects of a single type and write files to disk.
    pub async fn export_documentation(
        &self,
        type_id: &str,
        ids: Option<Vec<String>>,
        out_dir: &str,
        format: DocFormat,
    ) -> CoreResult<DocExportResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let target_ids = match ids {
            Some(v) if !v.is_empty() => v,
            _ => self.list_objects(type_id, None).await?.items.into_iter().map(|i| i.id).collect(),
        };
        let dir = std::path::Path::new(out_dir).join(&t.title);
        std::fs::create_dir_all(&dir)?;
        let mut files = Vec::new();
        for id in target_ids {
            let doc = self.document_object(type_id, &id).await?;
            let filename = format!("{}.{}", sanitize(&doc.name), format.extension());
            let path = dir.join(&filename);
            std::fs::write(&path, format.render(&doc))?;
            files.push(DocExportedFile {
                id,
                name: doc.name,
                path: path.to_string_lossy().to_string(),
                format,
            });
        }
        Ok(DocExportResult { directory: dir.to_string_lossy().to_string(), files })
    }

    // ---- Copy / Clone -----------------------------------------------------

    /// Copy (clone) a single object under a new name. Assignments are **not**
    /// copied. Defaults to a dry run; a real create only happens when `apply`
    /// is true *and* writes are explicitly enabled via `INTUNE_ALLOW_WRITES=1`.
    pub async fn copy_object(&self, type_id: &str, id: &str, new_name: Option<String>, apply: bool) -> CoreResult<CopyResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let detail = self.get_object(type_id, id).await?;
        let source_name = detail.name.clone();
        let new_name = new_name.unwrap_or_else(|| format!("{source_name} - Copy"));

        // Strip server-generated / read-only fields and assignments, then rename.
        let mut payload = clean_for_import(detail.object.clone());
        set_name(&mut payload, &t.name_property, &new_name);

        let target_api = graph::url(&t.api, None);
        let created = if apply {
            self.create_object(&target_api, &payload).await?
        } else {
            None
        };

        Ok(CopyResult {
            type_id: t.id.clone(),
            source_id: id.to_string(),
            source_name,
            new_name,
            payload,
            created: created.clone(),
            applied: apply && created.is_some(),
            target_api,
        })
    }

    /// Copy every object of a type whose name contains `pattern`
    /// (case-insensitive). Each copy is named by `name_template`, where the
    /// token `{name}` is replaced with the source name (default `{name} - Copy`).
    pub async fn copy_by_pattern(
        &self,
        type_id: &str,
        pattern: &str,
        name_template: Option<String>,
        apply: bool,
    ) -> CoreResult<CopyBatchResult> {
        let t = catalog::find(type_id).ok_or_else(|| CoreError::UnknownType(type_id.into()))?;
        let template = name_template.unwrap_or_else(|| "{name} - Copy".to_string());
        let matches = self.list_objects(type_id, Some(pattern)).await?;
        let mut copies = Vec::new();
        for item in matches.items {
            let new_name = template.replace("{name}", &item.name);
            let copy = self.copy_object(type_id, &item.id, Some(new_name), apply).await?;
            copies.push(copy);
        }
        Ok(CopyBatchResult { type_id: t.id.clone(), pattern: pattern.to_string(), applied: apply, copies })
    }

    /// POST a create to Graph, double-gated behind `INTUNE_ALLOW_WRITES=1`.
    async fn create_object(&self, target_api: &str, payload: &Value) -> CoreResult<Option<Value>> {
        if std::env::var("INTUNE_ALLOW_WRITES").ok().as_deref() != Some("1") {
            return Err(CoreError::Other(
                "Writes are disabled. Set INTUNE_ALLOW_WRITES=1 to allow creating objects.".into(),
            ));
        }
        let token = self.token().await?;
        let resp = self.http.post(target_api).bearer_auth(&token).json(payload).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(CoreError::Graph { status: status.as_u16(), message: body });
        }
        Ok(Some(serde_json::from_str(&body).unwrap_or(Value::Null)))
    }

    // ---- Bulk operations --------------------------------------------------

    /// Export several object types at once into `out_dir`, one subfolder per
    /// type (read-only against the tenant).
    pub async fn bulk_export(&self, type_ids: Vec<String>, out_dir: &str) -> CoreResult<BulkExportResult> {
        let mut results = Vec::new();
        let mut total_files = 0;
        for type_id in &type_ids {
            let res = self.export(type_id, None, out_dir).await?;
            total_files += res.files.len();
            results.push(res);
        }
        Ok(BulkExportResult { root: out_dir.to_string(), total_files, results })
    }

    /// Import an exported folder tree (`root/<Type Title>/<name>.json`),
    /// processing types in dependency order. Defaults to a dry run; real
    /// creates are double-gated behind `INTUNE_ALLOW_WRITES=1`.
    pub async fn bulk_import(&self, root_dir: &str, dry_run: bool) -> CoreResult<BulkImportResult> {
        let mut typed_files = scan_export_tree(root_dir)?;
        // Dependency order, then by type title for determinism.
        typed_files.sort_by(|a, b| {
            catalog::import_priority(&a.0.id)
                .cmp(&catalog::import_priority(&b.0.id))
                .then_with(|| a.0.title.cmp(&b.0.title))
        });
        let order: Vec<String> = {
            let mut seen = Vec::new();
            for (t, _) in &typed_files {
                if !seen.contains(&t.id) {
                    seen.push(t.id.clone());
                }
            }
            seen
        };

        let mut items = Vec::new();
        for (t, path) in typed_files {
            let file = path.to_string_lossy().to_string();
            let text = match std::fs::read_to_string(&path) {
                Ok(x) => x,
                Err(e) => {
                    items.push(BulkImportItem {
                        type_id: t.id.clone(),
                        type_title: t.title.clone(),
                        file,
                        name: String::new(),
                        payload: Value::Null,
                        created: None,
                        error: Some(e.to_string()),
                    });
                    continue;
                }
            };
            let parsed: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    items.push(BulkImportItem {
                        type_id: t.id.clone(),
                        type_title: t.title.clone(),
                        file,
                        name: String::new(),
                        payload: Value::Null,
                        created: None,
                        error: Some(format!("parse: {e}")),
                    });
                    continue;
                }
            };
            let name = display_name(&parsed, &t.name_property);
            let payload = clean_for_import(parsed);
            let target_api = graph::url(&t.api, None);
            let (created, error) = if dry_run {
                (None, None)
            } else {
                match self.create_object(&target_api, &payload).await {
                    Ok(c) => (c, None),
                    Err(e) => (None, Some(ErrorBody::from(&e).message)),
                }
            };
            items.push(BulkImportItem {
                type_id: t.id.clone(),
                type_title: t.title.clone(),
                file,
                name,
                payload,
                created,
                error,
            });
        }
        Ok(BulkImportResult { root: root_dir.to_string(), dry_run, order, items })
    }

    /// Compare an exported folder tree against the live tenant, matching each
    /// file to a live object by name (read-only).
    pub async fn bulk_compare(&self, root_dir: &str) -> CoreResult<BulkCompareResult> {
        let typed_files = scan_export_tree(root_dir)?;
        let mut items = Vec::new();
        // Cache live listings per type to avoid repeated Graph calls.
        let mut cache: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
        for (t, path) in typed_files {
            let file = path.to_string_lossy().to_string();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let file_obj: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    items.push(BulkCompareItem {
                        type_id: t.id.clone(),
                        type_title: t.title.clone(),
                        file,
                        name: String::new(),
                        matched: false,
                        identical: false,
                        added: 0,
                        removed: 0,
                        changed: 0,
                        error: Some(format!("parse: {e}")),
                    });
                    continue;
                }
            };
            let name = display_name(&file_obj, &t.name_property);

            if !cache.contains_key(&t.id) {
                let listed = self
                    .list_objects(&t.id, None)
                    .await
                    .map(|r| r.items.into_iter().map(|i| (i.name, i.id)).collect())
                    .unwrap_or_default();
                cache.insert(t.id.clone(), listed);
            }
            let live_id = cache
                .get(&t.id)
                .and_then(|list| list.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)).map(|(_, id)| id.clone()));

            match live_id {
                None => items.push(BulkCompareItem {
                    type_id: t.id.clone(),
                    type_title: t.title.clone(),
                    file,
                    name,
                    matched: false,
                    identical: false,
                    added: 0,
                    removed: 0,
                    changed: 0,
                    error: None,
                }),
                Some(id) => {
                    let detail = self.get_object(&t.id, &id).await?;
                    let cmp = compare::compare(&detail.object, &file_obj);
                    items.push(BulkCompareItem {
                        type_id: t.id.clone(),
                        type_title: t.title.clone(),
                        file,
                        name,
                        matched: true,
                        identical: cmp.identical,
                        added: cmp.added,
                        removed: cmp.removed,
                        changed: cmp.changed,
                        error: None,
                    });
                }
            }
        }
        Ok(BulkCompareResult { root: root_dir.to_string(), items })
    }

    // ---- Compare ----------------------------------------------------------

    pub async fn compare_to_file(&self, type_id: &str, id: &str, file_path: &str) -> CoreResult<compare::CompareResult> {
        let detail = self.get_object(type_id, id).await?;
        let text = std::fs::read_to_string(file_path)?;
        let file_obj: Value = serde_json::from_str(&text).map_err(|e| CoreError::Other(format!("parse compare file: {e}")))?;
        Ok(compare::compare(&detail.object, &file_obj))
    }
}

fn display_name(obj: &Value, name_property: &str) -> String {
    for key in [name_property, "displayName", "name", "fileName", "id"] {
        if let Some(s) = obj.get(key).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    "(unnamed)".to_string()
}

fn short_type(t: &str) -> String {
    t.trim_start_matches("#microsoft.graph.").to_string()
}

/// Walk an exported folder tree `root/<Type Title>/*.json` and map each JSON
/// file to its object type (by subfolder title). Files in unknown subfolders
/// are skipped.
fn scan_export_tree(root_dir: &str) -> CoreResult<Vec<(ObjectType, std::path::PathBuf)>> {
    let root = std::path::Path::new(root_dir);
    if !root.is_dir() {
        return Err(CoreError::Other(format!("not a directory: {root_dir}")));
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let sub = entry.path();
        if !sub.is_dir() {
            continue;
        }
        let title = sub.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let Some(t) = catalog::find_by_title(title) else { continue };
        for file in std::fs::read_dir(&sub)? {
            let file = file?;
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                out.push((t.clone(), path));
            }
        }
    }
    Ok(out)
}

/// Set the display name of a payload on the type's name property, with common
/// fallbacks so the rename lands regardless of the object's schema.
fn set_name(payload: &mut Value, name_property: &str, new_name: &str) {
    if let Some(map) = payload.as_object_mut() {
        let mut set_any = false;
        for key in [name_property, "displayName", "name"] {
            if map.contains_key(key) {
                map.insert(key.to_string(), Value::String(new_name.to_string()));
                set_any = true;
            }
        }
        if !set_any {
            map.insert(name_property.to_string(), Value::String(new_name.to_string()));
        }
    }
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed
    }
}

/// Read-only / server-generated properties removed before a create (import).
const READONLY_KEYS: &[&str] = &[
    "id",
    "createdDateTime",
    "lastModifiedDateTime",
    "version",
    "supportsScopeTags",
    "isAssigned",
    "@odata.context",
    "@odata.count",
    "@odata.nextLink",
    "assignments",
    "deviceStatusOverview",
    "userStatusOverview",
    "deviceStatuses",
    "userStatuses",
];

fn clean_for_import(mut v: Value) -> Value {
    if let Some(map) = v.as_object_mut() {
        for k in READONLY_KEYS {
            map.remove(*k);
        }
        // Strip Graph navigation-link annotations that break creates.
        let annotated: Vec<String> = map
            .keys()
            .filter(|k| k.contains("@odata.") && k.as_str() != "@odata.type")
            .cloned()
            .collect();
        for k in annotated {
            map.remove(&k);
        }
    }
    v
}
