use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    #[serde(rename = "isRegistered")]
    is_registered: bool,
}

#[derive(Clone)]
pub struct CommunityClient {
    http: reqwest::Client,
    base_url: String,
}

impl CommunityClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// Mirrors EventRegistrationRepository.findActiveRegistration(...) from
    /// the Java SlideDeckService.canAccess check.
    pub async fn has_active_registration(&self, event_id: &str, user_id: &str) -> bool {
        let url = format!(
            "{}/events/{event_id}/registration/{user_id}",
            self.base_url.trim_end_matches('/')
        );
        let mut request = self.http.get(&url);
        if let Some(request_id) = crate::request_id::current() {
            request = request.header(crate::request_id::HEADER_NAME, request_id);
        }
        match request.send().await {
            Ok(response) if response.status().is_success() => response
                .json::<RegistrationResponse>()
                .await
                .map(|r| r.is_registered)
                .unwrap_or(false),
            Ok(response) => {
                tracing::warn!(url, status = %response.status(), "unexpected response from community service");
                false
            }
            Err(err) => {
                tracing::error!(url, error = %err, "failed to reach community service");
                false
            }
        }
    }
}
