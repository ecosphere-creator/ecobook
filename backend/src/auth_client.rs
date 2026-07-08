use serde::Deserialize;

/// Subset of auth's UserDto shape actually needed here: resolving whether
/// an arbitrary user id is a platform owner, for the isEditor/canAccess
/// checks ported from SlideDeckService.
#[derive(Debug, Clone, Deserialize)]
pub struct Identity {
    pub role: String,
}

#[derive(Clone)]
pub struct AuthClient {
    http: reqwest::Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn is_platform_owner(&self, user_id: &str) -> bool {
        let url = format!("{}/auth/users/{}", self.base_url.trim_end_matches('/'), user_id);
        match self.http.get(&url).send().await {
            Ok(response) if response.status().is_success() => response
                .json::<Identity>()
                .await
                .map(|identity| identity.role.eq_ignore_ascii_case("owner"))
                .unwrap_or(false),
            Ok(response) => {
                if response.status() != reqwest::StatusCode::NOT_FOUND {
                    tracing::warn!(url, status = %response.status(), "unexpected response from auth service");
                }
                false
            }
            Err(err) => {
                tracing::error!(url, error = %err, "failed to reach auth service");
                false
            }
        }
    }
}
