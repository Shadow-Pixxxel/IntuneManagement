//! Human-readable documentation of Intune objects.
//!
//! Ports the spirit of the original tool's `Documentation*.psm1`: turn a policy
//! into readable settings. For Settings Catalog / Compliance V2 we resolve the
//! Graph `settingDefinitions` (display names + choice option labels) so the
//! output reads like the Intune portal. Other object types use a robust generic
//! property renderer. Output to Markdown, HTML and structured JSON.

use crate::catalog::ObjectType;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RowKind {
    /// A name/value setting row.
    Setting,
    /// A group/section header (value is empty).
    Group,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRow {
    pub name: String,
    pub value: String,
    pub level: u8,
    pub kind: RowKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocSection {
    pub title: String,
    pub rows: Vec<DocRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentedObject {
    pub type_id: String,
    pub type_title: String,
    pub object_id: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<DocSection>,
}

const META_KEYS: &[&str] = &[
    "id",
    "version",
    "createdDateTime",
    "lastModifiedDateTime",
    "supportsScopeTags",
    "isAssigned",
    "settingCount",
];

fn skip_key(k: &str) -> bool {
    k.starts_with("@odata") || k == "settings" || k == "assignments"
}

/// Format a camelCase / snake-case property key into a readable label.
pub fn format_key(key: &str) -> String {
    let key = key.trim_start_matches("@odata.");
    let mut out = String::new();
    let mut prev_lower = false;
    for (i, c) in key.chars().enumerate() {
        if c == '_' || c == '-' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        prev_lower = c.is_lowercase() || c.is_numeric();
    }
    out
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::Null => "—".to_string(),
        Value::Bool(b) => if *b { "Enabled".into() } else { "Disabled".into() },
        Value::String(s) => {
            if let Some(dt) = s.strip_suffix('Z') {
                if dt.contains('T') && dt.len() >= 19 {
                    return s.replace('T', " ").trim_end_matches('Z').to_string();
                }
            }
            s.clone()
        }
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

// ---- Settings Catalog resolver --------------------------------------------

fn build_defs(settings: &[Value]) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    for s in settings {
        if let Some(defs) = s.get("settingDefinitions").and_then(|v| v.as_array()) {
            for d in defs {
                if let Some(id) = d.get("id").and_then(|x| x.as_str()) {
                    m.entry(id.to_string()).or_insert_with(|| d.clone());
                }
            }
        }
    }
    m
}

fn def_display(defs: &HashMap<String, Value>, id: &str) -> String {
    defs.get(id)
        .and_then(|d| d.get("displayName"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| pretty_setting_id(id))
}

/// Fallback when a definition has no display name: humanise the last id segment.
fn pretty_setting_id(id: &str) -> String {
    let tail = id.rsplit(|c| c == '_' || c == '.').next().unwrap_or(id);
    format_key(tail)
}

fn resolve_choice(defs: &HashMap<String, Value>, def_id: &str, value_item_id: &str) -> String {
    if let Some(def) = defs.get(def_id) {
        if let Some(opts) = def.get("options").and_then(|v| v.as_array()) {
            for o in opts {
                if o.get("itemId").and_then(|v| v.as_str()) == Some(value_item_id) {
                    return o
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .unwrap_or_else(|| pretty_setting_id(value_item_id));
                }
            }
        }
    }
    pretty_setting_id(value_item_id)
}

fn walk_instance(defs: &HashMap<String, Value>, inst: &Value, level: u8, out: &mut Vec<DocRow>) {
    let odata = inst.get("@odata.type").and_then(|v| v.as_str()).unwrap_or("");
    let def_id = inst.get("settingDefinitionId").and_then(|v| v.as_str()).unwrap_or("");
    let name = def_display(defs, def_id);

    if odata.contains("ChoiceSettingInstance") {
        let val = inst.get("choiceSettingValue").and_then(|v| v.get("value")).and_then(|v| v.as_str()).unwrap_or("");
        out.push(DocRow { name, value: resolve_choice(defs, def_id, val), level, kind: RowKind::Setting });
        if let Some(children) = inst.get("choiceSettingValue").and_then(|v| v.get("children")).and_then(|v| v.as_array()) {
            for c in children.iter().filter(|c| c.is_object()) {
                walk_instance(defs, c, level + 1, out);
            }
        }
    } else if odata.contains("ChoiceSettingCollectionInstance") {
        out.push(DocRow { name, value: String::new(), level, kind: RowKind::Group });
        if let Some(vals) = inst.get("choiceSettingCollectionValue").and_then(|v| v.as_array()) {
            for cv in vals {
                let val = cv.get("value").and_then(|v| v.as_str()).unwrap_or("");
                out.push(DocRow { name: resolve_choice(defs, def_id, val), value: String::new(), level: level + 1, kind: RowKind::Setting });
                if let Some(children) = cv.get("children").and_then(|v| v.as_array()) {
                    for c in children.iter().filter(|c| c.is_object()) {
                        walk_instance(defs, c, level + 2, out);
                    }
                }
            }
        }
    } else if odata.contains("SimpleSettingCollectionInstance") {
        let joined = inst
            .get("simpleSettingCollectionValue")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| scalar_to_string(x.get("value").unwrap_or(&Value::Null))).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        out.push(DocRow { name, value: joined, level, kind: RowKind::Setting });
    } else if odata.contains("SimpleSettingInstance") {
        let v = inst.get("simpleSettingValue").and_then(|v| v.get("value")).cloned().unwrap_or(Value::Null);
        out.push(DocRow { name, value: scalar_to_string(&v), level, kind: RowKind::Setting });
    } else if odata.contains("GroupSettingCollectionInstance") {
        out.push(DocRow { name, value: String::new(), level, kind: RowKind::Group });
        if let Some(groups) = inst.get("groupSettingCollectionValue").and_then(|v| v.as_array()) {
            for gv in groups {
                if let Some(children) = gv.get("children").and_then(|v| v.as_array()) {
                    for c in children.iter().filter(|c| c.is_object()) {
                        walk_instance(defs, c, level + 1, out);
                    }
                }
            }
        }
    } else if odata.contains("GroupSettingInstance") {
        out.push(DocRow { name, value: String::new(), level, kind: RowKind::Group });
        if let Some(children) = inst.get("groupSettingValue").and_then(|v| v.get("children")).and_then(|v| v.as_array()) {
            for c in children.iter().filter(|c| c.is_object()) {
                walk_instance(defs, c, level + 1, out);
            }
        }
    } else {
        out.push(DocRow { name, value: format_key(odata.rsplit('.').next().unwrap_or("")), level, kind: RowKind::Setting });
    }
}

/// Build the "Configuration settings" section from a Settings Catalog `settings` list.
pub fn settings_catalog_section(settings: &[Value]) -> DocSection {
    let defs = build_defs(settings);
    let mut rows = Vec::new();
    for s in settings {
        if let Some(inst) = s.get("settingInstance") {
            walk_instance(&defs, inst, 0, &mut rows);
        }
    }
    DocSection { title: "Configuration settings".to_string(), rows }
}

// ---- Endpoint Security intent settings ------------------------------------

/// Humanise an intent `definitionId` such as
/// `deviceConfiguration--windows10EndpointProtectionConfiguration_defenderMonitorFileActivity`
/// into a readable label using the trailing setting segment.
fn intent_setting_name(definition_id: &str) -> String {
    let tail = definition_id.rsplit("--").next().unwrap_or(definition_id);
    let setting = tail.split_once('_').map(|(_, s)| s).unwrap_or(tail);
    format_key(setting)
}

fn intent_value_string(inst: &Value) -> String {
    // Prefer a structured `value`, then fall back to the raw `valueJson`.
    match inst.get("value") {
        Some(Value::Array(arr)) => {
            return arr
                .iter()
                .map(|x| scalar_to_string(x.get("value").unwrap_or(x)))
                .collect::<Vec<_>>()
                .join(", ");
        }
        Some(v) if !v.is_null() && !v.is_object() => return scalar_to_string(v),
        _ => {}
    }
    if let Some(vj) = inst.get("valueJson").and_then(|v| v.as_str()) {
        if vj != "null" && !vj.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<Value>(vj) {
                if parsed.is_string() || parsed.is_number() || parsed.is_boolean() {
                    return scalar_to_string(&parsed);
                }
            }
            return vj.to_string();
        }
    }
    "—".to_string()
}

/// Build the settings section for an Endpoint Security intent from its
/// `settings` collection (fetched from `/intents/{id}/settings`).
pub fn endpoint_security_section(settings: &[Value]) -> DocSection {
    let mut rows = Vec::new();
    for s in settings {
        let def_id = s.get("definitionId").and_then(|v| v.as_str()).unwrap_or("");
        if def_id.is_empty() {
            continue;
        }
        rows.push(DocRow {
            name: intent_setting_name(def_id),
            value: intent_value_string(s),
            level: 0,
            kind: RowKind::Setting,
        });
    }
    rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    DocSection { title: "Configuration settings".to_string(), rows }
}

// ---- Administrative Templates (ADMX) --------------------------------------

/// Build the settings section for an Administrative Template from its
/// `definitionValues` (each with an expanded `definition` and
/// `presentationValues`).
pub fn admx_section(definition_values: &[Value]) -> DocSection {
    let mut rows = Vec::new();
    for dv in definition_values {
        let def = dv.get("definition");
        let name = def
            .and_then(|d| d.get("displayName"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| "(unknown setting)".to_string());
        let category = def
            .and_then(|d| d.get("categoryPath"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('\\')
            .to_string();
        let enabled = dv.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let display = if category.is_empty() { name.clone() } else { format!("{category} \\ {name}") };
        rows.push(DocRow {
            name: display,
            value: if enabled { "Enabled".into() } else { "Disabled".into() },
            level: 0,
            kind: RowKind::Setting,
        });
        if let Some(pvs) = dv.get("presentationValues").and_then(|v| v.as_array()) {
            for pv in pvs {
                let label = pv
                    .get("presentation")
                    .and_then(|p| p.get("label"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .unwrap_or_else(|| "Value".to_string());
                let value = pv.get("value").cloned().unwrap_or(Value::Null);
                let value = if value.is_array() {
                    value.as_array().unwrap().iter().map(scalar_to_string).collect::<Vec<_>>().join(", ")
                } else {
                    scalar_to_string(&value)
                };
                rows.push(DocRow { name: format_key(&label), value, level: 1, kind: RowKind::Setting });
            }
        }
    }
    DocSection { title: "Configuration settings".to_string(), rows }
}

// ---- Generic property renderer --------------------------------------------

fn flatten(value: &Value, name: &str, level: u8, out: &mut Vec<DocRow>) {
    match value {
        Value::Object(map) => {
            out.push(DocRow { name: name.to_string(), value: String::new(), level, kind: RowKind::Group });
            for (k, v) in map {
                if skip_key(k) {
                    continue;
                }
                flatten(v, &format_key(k), level + 1, out);
            }
        }
        Value::Array(arr) => {
            if arr.iter().all(|x| !x.is_object() && !x.is_array()) {
                let joined = arr.iter().map(scalar_to_string).collect::<Vec<_>>().join(", ");
                out.push(DocRow { name: name.to_string(), value: joined, level, kind: RowKind::Setting });
            } else {
                out.push(DocRow { name: format!("{name} ({} items)", arr.len()), value: String::new(), level, kind: RowKind::Group });
                for (i, item) in arr.iter().enumerate() {
                    flatten(item, &format!("#{}", i + 1), level + 1, out);
                }
            }
        }
        other => out.push(DocRow { name: name.to_string(), value: scalar_to_string(other), level, kind: RowKind::Setting }),
    }
}

/// Generic settings section for object types without a settings-catalog schema.
pub fn generic_section(object: &Value) -> DocSection {
    let mut rows = Vec::new();
    if let Some(map) = object.as_object() {
        for (k, v) in map {
            if skip_key(k) || META_KEYS.contains(&k.as_str()) || k == "displayName" || k == "name" || k == "description" {
                continue;
            }
            flatten(v, &format_key(k), 0, &mut rows);
        }
    }
    DocSection { title: "Settings".to_string(), rows }
}

/// Metadata section (ids, timestamps, versions).
pub fn metadata_section(object: &Value) -> DocSection {
    let mut rows = Vec::new();
    if let Some(map) = object.as_object() {
        for key in META_KEYS {
            if let Some(v) = map.get(*key) {
                if !v.is_null() {
                    rows.push(DocRow { name: format_key(key), value: scalar_to_string(v), level: 0, kind: RowKind::Setting });
                }
            }
        }
    }
    DocSection { title: "Metadata".to_string(), rows }
}

/// Assignments section from a list of assignment objects.
pub fn assignments_section(assignments: &Value) -> Option<DocSection> {
    let arr = assignments.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for a in arr {
        let target = a.get("target").cloned().unwrap_or(Value::Null);
        let t = target.get("@odata.type").and_then(|v| v.as_str()).unwrap_or("").rsplit('.').next().unwrap_or("");
        let group = target.get("groupId").and_then(|v| v.as_str());
        let name = match group {
            Some(g) => format!("{} ({})", format_key(t), g),
            None => format_key(t),
        };
        let intent = a.get("intent").and_then(|v| v.as_str()).unwrap_or("");
        rows.push(DocRow { name, value: intent.to_string(), level: 0, kind: RowKind::Setting });
    }
    Some(DocSection { title: "Assignments".to_string(), rows })
}

// ---- Renderers -------------------------------------------------------------

fn indent_prefix(level: u8) -> String {
    "\u{00a0}\u{00a0}\u{00a0}\u{00a0}".repeat(level as usize)
}

impl DocumentedObject {
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", self.name));
        if let Some(d) = &self.description {
            if !d.is_empty() {
                s.push_str(&format!("{}\n\n", d));
            }
        }
        s.push_str(&format!("**Object type:** {}  \n", self.type_title));
        s.push_str(&format!("**Id:** `{}`\n\n", self.object_id));
        for section in &self.sections {
            if section.rows.is_empty() {
                continue;
            }
            s.push_str(&format!("## {}\n\n", section.title));
            s.push_str("| Setting | Value |\n| --- | --- |\n");
            for row in &section.rows {
                let name = format!("{}{}", indent_prefix(row.level), md_escape(&row.name));
                let name = if row.kind == RowKind::Group { format!("**{name}**") } else { name };
                s.push_str(&format!("| {} | {} |\n", name, md_escape(&row.value)));
            }
            s.push('\n');
        }
        s
    }

    pub fn to_html(&self) -> String {
        let mut body = String::new();
        body.push_str(&format!("<h1>{}</h1>", html_escape(&self.name)));
        if let Some(d) = &self.description {
            if !d.is_empty() {
                body.push_str(&format!("<p class=\"desc\">{}</p>", html_escape(d)));
            }
        }
        body.push_str(&format!(
            "<p class=\"meta\"><span><strong>Object type:</strong> {}</span><span><strong>Id:</strong> <code>{}</code></span></p>",
            html_escape(&self.type_title),
            html_escape(&self.object_id)
        ));
        for section in &self.sections {
            if section.rows.is_empty() {
                continue;
            }
            body.push_str(&format!("<h2>{}</h2>", html_escape(&section.title)));
            body.push_str("<table><thead><tr><th>Setting</th><th>Value</th></tr></thead><tbody>");
            for row in &section.rows {
                let cls = if row.kind == RowKind::Group { " class=\"group\"" } else { "" };
                let pad = row.level as usize * 20;
                body.push_str(&format!(
                    "<tr{cls}><td style=\"padding-left:{}px\">{}</td><td>{}</td></tr>",
                    pad + 12,
                    html_escape(&row.name),
                    html_escape(&row.value)
                ));
            }
            body.push_str("</tbody></table>");
        }
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>{}</style></head><body><main>{}</main></body></html>",
            html_escape(&self.name),
            HTML_CSS,
            body
        )
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const HTML_CSS: &str = r#"
:root{color-scheme:light dark}
*{box-sizing:border-box}
body{margin:0;background:#0b0f1a;color:#e5e9f2;font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;line-height:1.5}
main{max-width:960px;margin:0 auto;padding:40px 24px}
h1{font-size:26px;margin:0 0 8px;font-weight:650}
h2{font-size:17px;margin:32px 0 12px;padding-bottom:8px;border-bottom:1px solid #22283a;color:#a9b2c9}
.desc{color:#a9b2c9;margin:0 0 16px}
.meta{display:flex;gap:24px;flex-wrap:wrap;color:#8b93a9;font-size:13px;margin:0 0 8px}
code{background:#161c2b;padding:2px 6px;border-radius:6px;font-size:12px}
table{width:100%;border-collapse:collapse;font-size:13px;border:1px solid #22283a;border-radius:10px;overflow:hidden}
th{text-align:left;background:#141a29;color:#8b93a9;font-weight:600;padding:8px 12px;font-size:11px;text-transform:uppercase;letter-spacing:.04em}
td{padding:8px 12px;border-top:1px solid #1b2130;vertical-align:top}
td:last-child{color:#c9d2e6;font-weight:500;white-space:pre-wrap}
tr.group td{background:#101524;font-weight:650;color:#cdd5e8}
@media(prefers-color-scheme:light){body{background:#fff;color:#0f1222}h2{color:#5b607a;border-color:#e5e7eb}.desc,.meta{color:#5b607a}code{background:#f1f2f6}table{border-color:#e5e7eb}th{background:#f7f8fb;color:#5b607a}td{border-color:#eef0f4}tr.group td{background:#f5f6fa}}
"#;

/// Object types whose settings live in the Graph settings-catalog schema.
pub fn is_settings_catalog(type_id: &str) -> bool {
    matches!(type_id, "SettingsCatalog" | "CompliancePoliciesV2")
}

/// The settings endpoint path for a settings-catalog object.
pub fn settings_endpoint(t: &ObjectType, id: &str) -> String {
    format!("{}('{}')/settings?$expand=settingDefinitions", t.api, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_catalog_resolves_choice_labels() {
        let settings = json!([{
            "settingDefinitions": [
                {
                    "id": "device_vendor_msft_policy_allowarchivescanning",
                    "displayName": "Allow Archive Scanning",
                    "options": [
                        {"itemId": "device_vendor_msft_policy_allowarchivescanning_1", "displayName": "Allowed. Scans the archive files."}
                    ]
                }
            ],
            "settingInstance": {
                "@odata.type": "#microsoft.graph.deviceManagementConfigurationChoiceSettingInstance",
                "settingDefinitionId": "device_vendor_msft_policy_allowarchivescanning",
                "choiceSettingValue": {"value": "device_vendor_msft_policy_allowarchivescanning_1", "children": []}
            }
        }]);
        let section = settings_catalog_section(settings.as_array().unwrap());
        assert_eq!(section.rows.len(), 1);
        assert_eq!(section.rows[0].name, "Allow Archive Scanning");
        assert_eq!(section.rows[0].value, "Allowed. Scans the archive files.");
    }

    #[test]
    fn endpoint_security_intent_settings_render() {
        let settings = json!([
            {
                "@odata.type": "#microsoft.graph.deviceManagementBooleanSettingInstance",
                "definitionId": "deviceConfiguration--windows10EndpointProtectionConfiguration_defenderMonitorFileActivity",
                "value": true
            },
            {
                "@odata.type": "#microsoft.graph.deviceManagementIntegerSettingInstance",
                "definitionId": "deviceConfiguration--windows10EndpointProtectionConfiguration_defenderScanMaxCpu",
                "valueJson": "50"
            }
        ]);
        let section = endpoint_security_section(settings.as_array().unwrap());
        assert_eq!(section.rows.len(), 2);
        // sorted by name; "Defender Monitor File Activity" < "Defender Scan Max Cpu"
        assert_eq!(section.rows[0].name, "Defender Monitor File Activity");
        assert_eq!(section.rows[0].value, "Enabled");
        assert_eq!(section.rows[1].name, "Defender Scan Max Cpu");
        assert_eq!(section.rows[1].value, "50");
    }

    #[test]
    fn admx_definition_values_render() {
        let values = json!([
            {
                "enabled": true,
                "definition": {"displayName": "Allow Telemetry", "categoryPath": "\\Windows Components\\Data Collection"},
                "presentationValues": [
                    {"value": "3", "presentation": {"label": "Level"}}
                ]
            },
            {
                "enabled": false,
                "definition": {"displayName": "Turn off Autoplay", "categoryPath": "\\Windows Components\\AutoPlay Policies"},
                "presentationValues": []
            }
        ]);
        let section = admx_section(values.as_array().unwrap());
        assert_eq!(section.rows.len(), 3);
        assert_eq!(section.rows[0].name, "Windows Components\\Data Collection \\ Allow Telemetry");
        assert_eq!(section.rows[0].value, "Enabled");
        assert_eq!(section.rows[1].level, 1);
        assert_eq!(section.rows[1].name, "Level");
        assert_eq!(section.rows[1].value, "3");
        assert_eq!(section.rows[2].value, "Disabled");
    }
}
