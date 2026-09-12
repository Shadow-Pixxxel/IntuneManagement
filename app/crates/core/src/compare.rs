use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    pub path: String,
    pub kind: ChangeKind,
    /// Value in the live Intune object (left side).
    pub left: Option<Value>,
    /// Value in the exported file (right side).
    pub right: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareResult {
    pub identical: bool,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub differences: Vec<Difference>,
}

/// Keys that are expected to differ between a live object and an export
/// (server timestamps, ids, versions) and are ignored during comparison.
const IGNORE: &[&str] = &[
    "id",
    "createdDateTime",
    "lastModifiedDateTime",
    "version",
    "@odata.context",
    "supportsScopeTags",
    "isAssigned",
    "assignments",
];

/// Produce a property-level diff between a live Intune object (`left`) and an
/// exported file (`right`). Nested objects and arrays are compared recursively.
pub fn compare(left: &Value, right: &Value) -> CompareResult {
    let mut diffs = Vec::new();
    diff_value("", left, right, &mut diffs);
    let added = diffs.iter().filter(|d| d.kind == ChangeKind::Added).count();
    let removed = diffs.iter().filter(|d| d.kind == ChangeKind::Removed).count();
    let changed = diffs.iter().filter(|d| d.kind == ChangeKind::Changed).count();
    CompareResult { identical: diffs.is_empty(), added, removed, changed, differences: diffs }
}

fn is_ignored(path: &str) -> bool {
    let last = path.rsplit('.').next().unwrap_or(path);
    IGNORE.contains(&last)
}

fn diff_value(path: &str, left: &Value, right: &Value, out: &mut Vec<Difference>) {
    if is_ignored(path) {
        return;
    }
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            let mut keys: Vec<&String> = l.keys().chain(r.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let child = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                match (l.get(k), r.get(k)) {
                    (Some(lv), Some(rv)) => diff_value(&child, lv, rv, out),
                    (Some(lv), None) => {
                        if !is_ignored(&child) {
                            out.push(Difference { path: child, kind: ChangeKind::Removed, left: Some(lv.clone()), right: None });
                        }
                    }
                    (None, Some(rv)) => {
                        if !is_ignored(&child) {
                            out.push(Difference { path: child, kind: ChangeKind::Added, left: None, right: Some(rv.clone()) });
                        }
                    }
                    (None, None) => {}
                }
            }
        }
        (Value::Array(l), Value::Array(r)) => {
            if l != r {
                let max = l.len().max(r.len());
                for i in 0..max {
                    let child = format!("{path}[{i}]");
                    match (l.get(i), r.get(i)) {
                        (Some(lv), Some(rv)) => diff_value(&child, lv, rv, out),
                        (Some(lv), None) => out.push(Difference { path: child, kind: ChangeKind::Removed, left: Some(lv.clone()), right: None }),
                        (None, Some(rv)) => out.push(Difference { path: child, kind: ChangeKind::Added, left: None, right: Some(rv.clone()) }),
                        (None, None) => {}
                    }
                }
            }
        }
        _ => {
            if left != right {
                out.push(Difference { path: path.to_string(), kind: ChangeKind::Changed, left: Some(left.clone()), right: Some(right.clone()) });
            }
        }
    }
}
