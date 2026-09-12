use crate::error::{CoreError, CoreResult};
use serde_json::Value;
use std::time::Duration;

pub const GRAPH_BASE: &str = "https://graph.microsoft.com/beta";
const MAX_RETRIES: u32 = 5;

/// Perform a single authenticated GET, transparently retrying on HTTP 429 / 5xx.
pub async fn get(http: &reqwest::Client, token: &str, url: &str) -> CoreResult<Value> {
    let mut attempt = 0u32;
    loop {
        let resp = http.get(url).bearer_auth(token).send().await?;
        let status = resp.status();
        if status.is_success() {
            let body = resp.text().await?;
            return serde_json::from_str(&body).map_err(|e| CoreError::Other(format!("parse graph json: {e}")));
        }
        if (status.as_u16() == 429 || status.is_server_error()) && attempt < MAX_RETRIES {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or_else(|| 2u64.pow(attempt + 1));
            tokio::time::sleep(Duration::from_secs(retry_after.min(30))).await;
            attempt += 1;
            continue;
        }
        let body = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(String::from))
            .unwrap_or(body);
        return Err(CoreError::Graph { status: status.as_u16(), message });
    }
}

/// Build an absolute Graph URL from a relative path and optional query string.
pub fn url(path: &str, query: Option<&str>) -> String {
    let sep = if path.starts_with("http") { String::new() } else { GRAPH_BASE.to_string() };
    let base = format!("{sep}{path}");
    match query {
        Some(q) if !q.is_empty() => {
            let joiner = if base.contains('?') { '&' } else { '?' };
            format!("{base}{joiner}{q}")
        }
        _ => base,
    }
}

/// GET a collection, following `@odata.nextLink` until exhausted.
pub async fn get_all(http: &reqwest::Client, token: &str, first_url: &str) -> CoreResult<Vec<Value>> {
    let mut out = Vec::new();
    let mut next = Some(first_url.to_string());
    while let Some(u) = next {
        let page = get(http, token, &u).await?;
        if let Some(arr) = page.get("value").and_then(|v| v.as_array()) {
            out.extend(arr.iter().cloned());
        } else {
            // Single object endpoint returned instead of a collection.
            out.push(page);
            break;
        }
        next = page.get("@odata.nextLink").and_then(|v| v.as_str()).map(String::from);
    }
    Ok(out)
}
