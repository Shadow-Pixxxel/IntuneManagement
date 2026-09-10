use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const DEFAULT_TENANT: &str = "common";
/// Public client id shipped by the original IntuneManagement tool (Azure PowerShell style).
/// Used for the interactive device-code flow when the user does not provide their own app.
pub const DEFAULT_PUBLIC_APP_ID: &str = "d1ddf0e4-d672-4dae-b554-9d5bdfd93547";

const SCOPE_APP: &str = "https://graph.microsoft.com/.default";
const SCOPE_DELEGATED: &str = "https://graph.microsoft.com/.default offline_access";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMode {
    AppOnly,
    DeviceCode,
}

/// Active credentials + token cache. Lives only in the backend process, never the renderer.
#[derive(Debug, Clone)]
pub struct Session {
    pub mode: AuthMode,
    pub tenant_id: String,
    pub app_id: String,
    app_secret: Option<String>,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: OffsetDateTime,
    pub org_name: Option<String>,
    pub org_id: Option<String>,
}

/// Public status snapshot safe to send to the UI (no secrets or tokens).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub authenticated: bool,
    pub mode: Option<AuthMode>,
    pub tenant_id: Option<String>,
    pub app_id: Option<String>,
    pub org_name: Option<String>,
    pub org_id: Option<String>,
}

impl AuthStatus {
    pub fn signed_out() -> Self {
        AuthStatus { authenticated: false, mode: None, tenant_id: None, app_id: None, org_name: None, org_id: None }
    }
}

/// Payload returned when a device-code flow starts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: i64,
    pub message: String,
    pub tenant_id: String,
    pub app_id: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: i64,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

fn authority(tenant: &str) -> String {
    format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token")
}

impl Session {
    fn from_token(mode: AuthMode, tenant_id: String, app_id: String, app_secret: Option<String>, t: TokenResponse) -> Self {
        Session {
            mode,
            tenant_id,
            app_id,
            app_secret,
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            expires_at: OffsetDateTime::now_utc() + time::Duration::seconds(t.expires_in.max(60) - 30),
            org_name: None,
            org_id: None,
        }
    }

    pub fn status(&self) -> AuthStatus {
        AuthStatus {
            authenticated: true,
            mode: Some(self.mode),
            tenant_id: Some(self.tenant_id.clone()),
            app_id: Some(self.app_id.clone()),
            org_name: self.org_name.clone(),
            org_id: self.org_id.clone(),
        }
    }

    /// Return a valid access token, transparently refreshing if it has expired.
    pub async fn token(&mut self, http: &reqwest::Client) -> CoreResult<String> {
        if OffsetDateTime::now_utc() < self.expires_at {
            return Ok(self.access_token.clone());
        }
        match self.mode {
            AuthMode::AppOnly => {
                let secret = self.app_secret.clone().ok_or_else(|| CoreError::Auth("missing app secret".into()))?;
                let t = request_client_credentials(http, &self.tenant_id, &self.app_id, &secret).await?;
                self.access_token = t.access_token.clone();
                self.expires_at = OffsetDateTime::now_utc() + time::Duration::seconds(t.expires_in.max(60) - 30);
            }
            AuthMode::DeviceCode => {
                let refresh = self.refresh_token.clone().ok_or_else(|| CoreError::Auth("session expired, sign in again".into()))?;
                let t = request_refresh(http, &self.tenant_id, &self.app_id, &refresh).await?;
                self.access_token = t.access_token.clone();
                if t.refresh_token.is_some() {
                    self.refresh_token = t.refresh_token.clone();
                }
                self.expires_at = OffsetDateTime::now_utc() + time::Duration::seconds(t.expires_in.max(60) - 30);
            }
        }
        Ok(self.access_token.clone())
    }
}

async fn post_token(http: &reqwest::Client, tenant: &str, form: &[(&str, &str)]) -> CoreResult<TokenResponse> {
    let resp = http.post(authority(tenant)).form(form).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if status.is_success() {
        serde_json::from_str::<TokenResponse>(&body).map_err(|e| CoreError::Auth(format!("parse token: {e}")))
    } else if let Ok(err) = serde_json::from_str::<TokenError>(&body) {
        if err.error == "authorization_pending" || err.error == "slow_down" {
            Err(CoreError::AuthorizationPending)
        } else {
            Err(CoreError::Auth(err.error_description.unwrap_or(err.error)))
        }
    } else {
        Err(CoreError::Auth(format!("token endpoint {status}: {body}")))
    }
}

async fn request_client_credentials(http: &reqwest::Client, tenant: &str, app_id: &str, secret: &str) -> CoreResult<TokenResponse> {
    post_token(http, tenant, &[
        ("client_id", app_id),
        ("scope", SCOPE_APP),
        ("client_secret", secret),
        ("grant_type", "client_credentials"),
    ]).await
}

async fn request_refresh(http: &reqwest::Client, tenant: &str, app_id: &str, refresh: &str) -> CoreResult<TokenResponse> {
    post_token(http, tenant, &[
        ("client_id", app_id),
        ("scope", SCOPE_DELEGATED),
        ("refresh_token", refresh),
        ("grant_type", "refresh_token"),
    ]).await
}

/// App-only client-credentials sign in. Arguments fall back to env vars.
pub async fn login_app_only(
    http: &reqwest::Client,
    tenant_id: Option<String>,
    app_id: Option<String>,
    app_secret: Option<String>,
) -> CoreResult<Session> {
    let tenant_id = tenant_id.or_else(|| std::env::var("tenant_id").ok()).ok_or_else(|| CoreError::Auth("tenant_id required".into()))?;
    let app_id = app_id.or_else(|| std::env::var("app_id").ok()).ok_or_else(|| CoreError::Auth("app_id required".into()))?;
    let app_secret = app_secret.or_else(|| std::env::var("app_secret").ok()).ok_or_else(|| CoreError::Auth("app_secret required".into()))?;
    let t = request_client_credentials(http, &tenant_id, &app_id, &app_secret).await?;
    Ok(Session::from_token(AuthMode::AppOnly, tenant_id, app_id, Some(app_secret), t))
}

/// Begin the OAuth 2.0 device authorization grant (best fit for Linux/Wayland desktops).
pub async fn device_code_start(
    http: &reqwest::Client,
    tenant_id: Option<String>,
    app_id: Option<String>,
) -> CoreResult<DeviceCodeStart> {
    let tenant_id = tenant_id.filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_TENANT.to_string());
    let app_id = app_id.filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_PUBLIC_APP_ID.to_string());
    let url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/devicecode");
    let resp = http.post(&url).form(&[("client_id", app_id.as_str()), ("scope", SCOPE_DELEGATED)]).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(CoreError::Auth(format!("devicecode {status}: {body}")));
    }
    #[derive(Deserialize)]
    struct Dc {
        device_code: String,
        user_code: String,
        verification_uri: String,
        expires_in: i64,
        interval: i64,
        message: String,
    }
    let dc: Dc = serde_json::from_str(&body).map_err(|e| CoreError::Auth(format!("parse devicecode: {e}")))?;
    Ok(DeviceCodeStart {
        device_code: dc.device_code,
        user_code: dc.user_code,
        verification_uri: dc.verification_uri,
        expires_in: dc.expires_in,
        interval: dc.interval,
        message: dc.message,
        tenant_id,
        app_id,
    })
}

/// Poll once for a device-code token. Returns `AuthorizationPending` until the user approves.
pub async fn device_code_poll(
    http: &reqwest::Client,
    tenant_id: &str,
    app_id: &str,
    device_code: &str,
) -> CoreResult<Session> {
    let t = post_token(http, tenant_id, &[
        ("client_id", app_id),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ]).await?;
    Ok(Session::from_token(AuthMode::DeviceCode, tenant_id.to_string(), app_id.to_string(), None, t))
}
