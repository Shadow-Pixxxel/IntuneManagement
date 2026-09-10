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
use intune_core::{error::ErrorBody, Backend};
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
        .route("/api/export", post(export))
        .route("/api/import", post(import_file))
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
