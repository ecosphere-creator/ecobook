use std::env;

/// Placeholder secrets that have appeared in this codebase's example/default
/// config at some point. Refusing to start on these specifically (on top of
/// the general length check) catches the case where a real deployment
/// copy-pasted a default instead of generating its own secret.
const KNOWN_PLACEHOLDER_SECRETS: &[&str] = &[
    "your-secret-key-change-in-production",
    "change-this-secret",
    "secret",
];

/// HS512 wants a key at least as long as its output (64 bytes / 512 bits) or
/// the signature offers less security than the algorithm name implies. 32 is
/// enforced as a hard minimum; anything under 64 is allowed but warned about.
const MIN_JWT_SECRET_BYTES: usize = 32;
const RECOMMENDED_JWT_SECRET_BYTES: usize = 64;

#[derive(Clone)]
pub struct AppConfig {
    pub mongodb_uri: String,
    /// Must match the shared secret used by every other service in the
    /// estate -- this service only validates tokens issued by auth, it
    /// never issues its own.
    pub jwt_secret: String,
    pub server_port: u16,
    pub api_base_url: String,
    pub cors_allowed_origins: Vec<String>,
    /// Resolved by `eco configure` when these peers are composed into the
    /// same estate (see resolve_peer_base_urls in eco/configure.sh).
    pub auth_base_url: String,
    pub payments_base_url: String,
    pub community_base_url: String,
    /// Interim local-disk storage for slide images, until MinIO/S3 is
    /// provisioned for the estate.
    pub storage_local_path: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let server_port = env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_default();
        if jwt_secret.is_empty() {
            anyhow::bail!(
                "JWT_SECRET is not set. Refusing to start with no way to validate tokens -- \
                 this must match the estate's shared secret (see auth's .env)."
            );
        }
        if KNOWN_PLACEHOLDER_SECRETS.contains(&jwt_secret.as_str()) {
            anyhow::bail!(
                "JWT_SECRET is set to a known placeholder value. Refusing to start -- \
                 use the estate's real shared secret."
            );
        }
        if jwt_secret.len() < MIN_JWT_SECRET_BYTES {
            anyhow::bail!(
                "JWT_SECRET is only {} bytes; refusing to start with fewer than {} \
                 for HS512 validation.",
                jwt_secret.len(),
                MIN_JWT_SECRET_BYTES
            );
        }
        if jwt_secret.len() < RECOMMENDED_JWT_SECRET_BYTES {
            tracing::warn!(
                bytes = jwt_secret.len(),
                recommended = RECOMMENDED_JWT_SECRET_BYTES,
                "JWT_SECRET is shorter than recommended for HS512"
            );
        }

        Ok(Self {
            mongodb_uri: env::var("MONGODB_URI")
                .unwrap_or_else(|_| "mongodb://localhost:27017/slides_dev".to_string()),
            jwt_secret,
            server_port,
            api_base_url: env::var("API_BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{server_port}/api")),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            auth_base_url: env::var("AUTH_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9001/api".to_string()),
            payments_base_url: env::var("PAYMENTS_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9014/api".to_string()),
            community_base_url: env::var("COMMUNITY_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9011/api".to_string()),
            storage_local_path: env::var("STORAGE_LOCAL_PATH")
                .unwrap_or_else(|_| "./storage".to_string()),
        })
    }
}
