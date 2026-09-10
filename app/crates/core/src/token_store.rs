//! Optional OS keyring persistence for the delegated (device-code) refresh
//! token, so the desktop app can restore a session across restarts.
//!
//! This is strictly best-effort: on headless or locked-down systems where no
//! OS keyring backend is available, every operation degrades gracefully to a
//! no-op and the app keeps the session in-process only. Access tokens and
//! client secrets are never persisted here — only the refresh token, which the
//! OS keyring stores encrypted at rest.

use serde::{Deserialize, Serialize};

const SERVICE: &str = "intune-manager-crossplatform";
const ACCOUNT: &str = "device-code-session";

/// The minimal, non-secret-secret material needed to rehydrate a delegated
/// session. (`refresh_token` is sensitive; the OS keyring protects it.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuth {
    pub tenant_id: String,
    pub app_id: String,
    pub refresh_token: String,
}

fn entry() -> Option<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).ok()
}

/// Persist the refresh token. Returns `true` on success, `false` if no keyring
/// backend is available (the caller should just continue in-process).
pub fn save(auth: &PersistedAuth) -> bool {
    let Some(entry) = entry() else { return false };
    let Ok(json) = serde_json::to_string(auth) else { return false };
    match entry.set_password(&json) {
        Ok(()) => true,
        Err(e) => {
            tracing::debug!("keyring save unavailable: {e}");
            false
        }
    }
}

/// Load a persisted refresh token, if any keyring backend has one stored.
pub fn load() -> Option<PersistedAuth> {
    let entry = entry()?;
    match entry.get_password() {
        Ok(json) => serde_json::from_str(&json).ok(),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            tracing::debug!("keyring load unavailable: {e}");
            None
        }
    }
}

/// Remove any persisted refresh token (called on sign out).
pub fn clear() {
    if let Some(entry) = entry() {
        let _ = entry.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_clear_roundtrip_or_graceful_noop() {
        let auth = PersistedAuth {
            tenant_id: "tenant-123".into(),
            app_id: "app-456".into(),
            refresh_token: "refresh-secret".into(),
        };
        // On systems with a keyring backend this round-trips; otherwise `save`
        // returns false and we simply assert the graceful no-op contract.
        if save(&auth) {
            let loaded = load().expect("saved entry should load");
            assert_eq!(loaded.tenant_id, "tenant-123");
            assert_eq!(loaded.app_id, "app-456");
            assert_eq!(loaded.refresh_token, "refresh-secret");
            clear();
            assert!(load().is_none(), "cleared entry should be gone");
        } else {
            // No backend available: load must also be a graceful None.
            assert!(load().is_none());
        }
    }
}
