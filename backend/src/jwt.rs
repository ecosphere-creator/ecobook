use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// Mirrors the claims auth issues (HS512, sub/username/role/iat/exp). This
/// service only ever validates tokens -- issuing them is auth's job alone.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn validate_token(secret: &str, token: &str) -> Option<Claims> {
    let validation = Validation::new(Algorithm::HS512);
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
        .ok()
        .map(|data| data.claims)
}
