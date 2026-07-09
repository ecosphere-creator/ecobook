use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AccessResponse {
    #[serde(rename = "hasAccess")]
    has_access: bool,
}

#[derive(Clone)]
pub struct PaymentsClient {
    http: reqwest::Client,
    base_url: String,
}

impl PaymentsClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    /// Mirrors PaymentRepository.findByUserIdAndSlideDeckIdAndStatus(...,
    /// "completed") from the Java SlideDeckService.canAccess check.
    pub async fn has_completed_slide_payment(&self, user_id: &str, slide_deck_id: &str) -> bool {
        let url = format!(
            "{}/payments/access/slide-deck/{slide_deck_id}/user/{user_id}",
            self.base_url.trim_end_matches('/')
        );
        let mut request = self.http.get(&url);
        if let Some(request_id) = crate::request_id::current() {
            request = request.header(crate::request_id::HEADER_NAME, request_id);
        }
        match request.send().await {
            Ok(response) if response.status().is_success() => response
                .json::<AccessResponse>()
                .await
                .map(|r| r.has_access)
                .unwrap_or(false),
            Ok(response) => {
                tracing::warn!(url, status = %response.status(), "unexpected response from payments service");
                false
            }
            Err(err) => {
                tracing::error!(url, error = %err, "failed to reach payments service");
                false
            }
        }
    }
}
