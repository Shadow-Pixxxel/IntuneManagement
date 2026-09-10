//! Local HTTP sidecar exposing the `intune_core` backend.
//!
//! Used for (a) browser-based development of the React frontend (Vite proxies
//! `/api` here) and (b) headless / DevOps automation. The Tauri desktop app
//! uses the same `Backend` via native commands instead of HTTP.
//!
//! It binds to 127.0.0.1 only; tokens and secrets stay inside this process.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use intune_core::{error::ErrorBody, Backend, DocFormat};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

type Shared = Arc<Backend>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())).init();

    let backend = Arc::new(Backend::new());
    let app = Router::new()
        .route("/api/health", get(|| async { Json(json!({"ok": true})) }))
        .route("/api/status", get(status))
        .route("/api/auth/app-only", post(login_app_only))
        .route("/api/auth/device/start", post(device_start))
        .route("/api/auth/device/poll", post(device_poll))
        .route("/api/auth/logout", post(logout))
        .route("/api/catalog", get(catalog))
        .route("/api/objects/:type_id", get(list_objects))
        .route("/api/objects/:type_id/:id", get(get_object))
        .route("/api/document/:type_id/:id", get(document_object))
        .route("/api/document/export", post(export_documentation))
        .route("/api/export", post(export))
        .route("/api/import", post(import_file))
        .route("/api/copy", post(copy_object))
        .route("/api/copy/pattern", post(copy_by_pattern))
        .route("/api/bulk/export", post(bulk_export))
        .route("/api/bulk/import", post(bulk_import))
        .route("/api/bulk/compare", post(bulk_compare))
        .route("/api/compare", post(compare))
        .layer(CorsLayer::permissive())
        .with_state(backend);

    let port: u16 = std::env::var("INTUNE_SERVER_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    tracing::info!("intune-server listening on http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}

fn err(e: intune_core::error::CoreError) -> (StatusCode, Json<ErrorBody>) {
    use intune_core::error::CoreError::*;
    let status = match &e {
        NotAuthenticated => StatusCode::UNAUTHORIZED,
        AuthorizationPending => StatusCode::ACCEPTED,
        UnknownType(_) => StatusCode::NOT_FOUND,
        Graph { status, .. } => StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
        _ => StatusCode::BAD_REQUEST,
    };
    (status, Json(ErrorBody::from(&e)))
}

async fn status(State(b): State<Shared>) -> impl IntoResponse {
    Json(b.status().await)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppOnlyBody {
    tenant_id: Option<String>,
    app_id: Option<String>,
    app_secret: Option<String>,
}

async fn login_app_only(State(b): State<Shared>, Json(body): Json<AppOnlyBody>) -> impl IntoResponse {
    match b.login_app_only(body.tenant_id, body.app_id, body.app_secret).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStartBody {
    tenant_id: Option<String>,
    app_id: Option<String>,
}

async fn device_start(State(b): State<Shared>, Json(body): Json<DeviceStartBody>) -> impl IntoResponse {
    match b.device_start(body.tenant_id, body.app_id).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevicePollBody {
    tenant_id: String,
    app_id: String,
    device_code: String,
}

async fn device_poll(State(b): State<Shared>, Json(body): Json<DevicePollBody>) -> impl IntoResponse {
    match b.device_poll(&body.tenant_id, &body.app_id, &body.device_code).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn logout(State(b): State<Shared>) -> impl IntoResponse {
    b.logout().await;
    Json(json!({"ok": true}))
}

async fn catalog(State(b): State<Shared>) -> impl IntoResponse {
    Json(b.catalog())
}

#[derive(Deserialize)]
struct SearchQuery {
    search: Option<String>,
}

async fn list_objects(State(b): State<Shared>, Path(type_id): Path<String>, Query(q): Query<SearchQuery>) -> impl IntoResponse {
    match b.list_objects(&type_id, q.search.as_deref()).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn get_object(State(b): State<Shared>, Path((type_id, id)): Path<(String, String)>) -> impl IntoResponse {
    match b.get_object(&type_id, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

async fn document_object(State(b): State<Shared>, Path((type_id, id)): Path<(String, String)>) -> impl IntoResponse {
    match b.document_object(&type_id, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocExportBody {
    type_id: String,
    ids: Option<Vec<String>>,
    out_dir: String,
    #[serde(default = "default_doc_format")]
    format: DocFormat,
}
fn default_doc_format() -> DocFormat {
    DocFormat::Markdown
}

async fn export_documentation(State(b): State<Shared>, Json(body): Json<DocExportBody>) -> impl IntoResponse {
    match b.export_documentation(&body.type_id, body.ids, &body.out_dir, body.format).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportBody {
    type_id: String,
    ids: Option<Vec<String>>,
    out_dir: String,
}

async fn export(State(b): State<Shared>, Json(body): Json<ExportBody>) -> impl IntoResponse {
    match b.export(&body.type_id, body.ids, &body.out_dir).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportBody {
    type_id: String,
    file_path: String,
    #[serde(default = "default_true")]
    dry_run: bool,
}
fn default_true() -> bool {
    true
}

async fn import_file(State(b): State<Shared>, Json(body): Json<ImportBody>) -> impl IntoResponse {
    match b.import_file(&body.type_id, &body.file_path, body.dry_run).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopyBody {
    type_id: String,
    id: String,
    new_name: Option<String>,
    #[serde(default)]
    apply: bool,
}

async fn copy_object(State(b): State<Shared>, Json(body): Json<CopyBody>) -> impl IntoResponse {
    match b.copy_object(&body.type_id, &body.id, body.new_name, body.apply).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopyPatternBody {
    type_id: String,
    pattern: String,
    name_template: Option<String>,
    #[serde(default)]
    apply: bool,
}

async fn copy_by_pattern(State(b): State<Shared>, Json(body): Json<CopyPatternBody>) -> impl IntoResponse {
    match b.copy_by_pattern(&body.type_id, &body.pattern, body.name_template, body.apply).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompareBody {
    type_id: String,
    id: String,
    file_path: String,
}

async fn compare(State(b): State<Shared>, Json(body): Json<CompareBody>) -> impl IntoResponse {
    match b.compare_to_file(&body.type_id, &body.id, &body.file_path).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkExportBody {
    type_ids: Vec<String>,
    out_dir: String,
}

async fn bulk_export(State(b): State<Shared>, Json(body): Json<BulkExportBody>) -> impl IntoResponse {
    match b.bulk_export(body.type_ids, &body.out_dir).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkImportBody {
    root_dir: String,
    #[serde(default = "default_true")]
    dry_run: bool,
}

async fn bulk_import(State(b): State<Shared>, Json(body): Json<BulkImportBody>) -> impl IntoResponse {
    match b.bulk_import(&body.root_dir, body.dry_run).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkCompareBody {
    root_dir: String,
}

async fn bulk_compare(State(b): State<Shared>, Json(body): Json<BulkCompareBody>) -> impl IntoResponse {
    match b.bulk_compare(&body.root_dir).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => err(e).into_response(),
    }
}
