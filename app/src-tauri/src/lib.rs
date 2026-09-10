//! Tauri desktop shell. Thin `#[tauri::command]` wrappers over `intune_core::Backend`.
//! Tokens and secrets live only in this Rust process, never the renderer.

use intune_core::{
    auth::{AuthStatus, DeviceCodeStart},
    catalog::ObjectType,
    compare::CompareResult,
    error::CoreError,
    Backend, ExportResult, ImportResult, ListResult, ObjectDetail,
};
use std::sync::Arc;
use tauri::State;

type B = State<'_, Arc<Backend>>;

#[tauri::command]
async fn auth_status(backend: B) -> AuthStatus {
    backend.status().await
}

#[tauri::command]
async fn login_app_only(
    backend: B,
    tenant_id: Option<String>,
    app_id: Option<String>,
    app_secret: Option<String>,
) -> Result<AuthStatus, CoreError> {
    backend.login_app_only(tenant_id, app_id, app_secret).await
}

#[tauri::command]
async fn device_start(backend: B, tenant_id: Option<String>, app_id: Option<String>) -> Result<DeviceCodeStart, CoreError> {
    backend.device_start(tenant_id, app_id).await
}

#[tauri::command]
async fn device_poll(backend: B, tenant_id: String, app_id: String, device_code: String) -> Result<AuthStatus, CoreError> {
    backend.device_poll(&tenant_id, &app_id, &device_code).await
}

#[tauri::command]
async fn logout(backend: B) {
    backend.logout().await
}

#[tauri::command]
fn catalog(backend: B) -> Vec<ObjectType> {
    backend.catalog()
}

#[tauri::command]
async fn list_objects(backend: B, type_id: String, search: Option<String>) -> Result<ListResult, CoreError> {
    backend.list_objects(&type_id, search.as_deref()).await
}

#[tauri::command]
async fn get_object(backend: B, type_id: String, id: String) -> Result<ObjectDetail, CoreError> {
    backend.get_object(&type_id, &id).await
}

#[tauri::command]
async fn export(backend: B, type_id: String, ids: Option<Vec<String>>, out_dir: String) -> Result<ExportResult, CoreError> {
    backend.export(&type_id, ids, &out_dir).await
}

#[tauri::command]
async fn import_file(backend: B, type_id: String, file_path: String, dry_run: bool) -> Result<ImportResult, CoreError> {
    backend.import_file(&type_id, &file_path, dry_run).await
}

#[tauri::command]
async fn compare_to_file(backend: B, type_id: String, id: String, file_path: String) -> Result<CompareResult, CoreError> {
    backend.compare_to_file(&type_id, &id, &file_path).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Backend::new()))
        .invoke_handler(tauri::generate_handler![
            auth_status,
            login_app_only,
            device_start,
            device_poll,
            logout,
            catalog,
            list_objects,
            get_object,
            export,
            import_file,
            compare_to_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
