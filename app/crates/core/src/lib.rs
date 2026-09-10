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

use auth::{AuthStatus, DeviceCodeStart, Session};
use documentation::DocumentedObject;
use error::{CoreError, CoreResult};
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
        let status = session.status();
        *self.session.lock().await = Some(session);
        Ok(status)
    }

    pub async fn logout(&self) {
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
            if let Some(filter) = &t.odata_type_filter {
                let ot = obj.get("@odata.type").and_then(|v| v.as_str()).unwrap_or("");
                if !ot.to_lowercase().contains(&filter.to_lowercase()) {
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
        let object = graph::get(&self.http, &token, &url).await?;
        let name = display_name(&object, &t.name_property);
        let assignments = if t.assignments {
            let aurl = graph::url(&format!("{}/{}/assignments", t.api, id), None);
            graph::get(&self.http, &token, &aurl).await.ok().and_then(|v| v.get("value").cloned())
        } else {
            None
        };
        Ok(ObjectDetail { id: id.to_string(), name, object, assignments })
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
            let token = self.token().await?;
            let resp = self
                .http
                .post(&target_api)
                .bearer_auth(&token)
                .json(&payload)
                .send()
                .await?;
            let status = resp.status();
            let body = resp.text().await?;
            if !status.is_success() {
                return Err(CoreError::Graph { status: status.as_u16(), message: body });
            }
            Some(serde_json::from_str(&body).unwrap_or(Value::Null))
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
