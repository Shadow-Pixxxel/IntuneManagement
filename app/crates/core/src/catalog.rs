use serde::{Deserialize, Serialize};

/// A single Intune / Azure AD object type the app can browse, export and import.
/// This mirrors the object definitions from the original IntuneManagement tool
/// (`Extensions/EndpointManager.psm1`) but is decoupled from PowerShell/WPF.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectType {
    pub id: String,
    pub title: String,
    /// Graph beta API path, e.g. `/deviceManagement/deviceConfigurations`.
    pub api: String,
    pub group: String,
    pub group_order: i64,
    /// Property holding the human friendly name (usually `displayName`, sometimes `name`).
    pub name_property: String,
    /// Extra `$filter`/`$select` query fragment used when listing.
    pub query_list: Option<String>,
    /// `$expand` clause used when fetching a single object.
    pub expand: Option<String>,
    /// Whether the type supports group assignments.
    pub assignments: bool,
    /// For endpoints that multiplex several types, filter list results by `@odata.type` substring.
    pub odata_type_filter: Option<String>,
    pub icon: String,
}

const CATALOG_JSON: &str = include_str!("../catalog.json");

/// The full catalog, parsed once from the embedded JSON.
pub fn catalog() -> Vec<ObjectType> {
    serde_json::from_str(CATALOG_JSON).expect("embedded catalog.json is valid")
}

/// Find an object type by its stable id.
pub fn find(id: &str) -> Option<ObjectType> {
    catalog().into_iter().find(|t| t.id == id)
}
