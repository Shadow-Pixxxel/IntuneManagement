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
    /// For endpoints that multiplex several types, keep only list results whose
    /// `@odata.type` contains one of these (comma-separated) substrings.
    pub odata_type_filter: Option<String>,
    /// For multiplexed endpoints, drop list results whose `@odata.type` contains
    /// one of these (comma-separated) substrings. Applied after `odata_type_filter`.
    #[serde(default)]
    pub odata_type_exclude: Option<String>,
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

/// Find an object type by its human title (used to map export subfolders back to types).
pub fn find_by_title(title: &str) -> Option<ObjectType> {
    catalog().into_iter().find(|t| t.title.eq_ignore_ascii_case(title))
}

/// Dependency import priority: lower values are imported first. Objects that are
/// referenced by others (scope tags, filters, scripts, apps) come before the
/// policies that use them; aggregates like Policy Sets come last.
pub fn import_priority(type_id: &str) -> u32 {
    match type_id {
        "ScopeTags" => 0,
        "RoleDefinitions" => 5,
        "AssignmentFilters" => 10,
        "DeviceCategories" => 15,
        "Locations" | "NamedLocations" => 20,
        "Notifications" | "TermsOfUse" | "TermsAndConditions" => 25,
        "ReusableSettings" => 30,
        "ComplianceScripts" | "DeviceHealthScripts" | "PowerShellScripts" | "MacScripts" | "MacCustomAttributes" => 35,
        "AuthenticationContext" | "AuthenticationStrengths" => 40,
        "Applications" => 50,
        "AppConfigurationManagedApp" | "AppConfigurationManagedDevice" | "AppProtection" => 110,
        "PolicySets" => 900,
        "ConditionalAccess" => 950,
        _ => 100,
    }
}
