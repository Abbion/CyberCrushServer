use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct ResponseStatus {
    pub success: bool,
    pub status_message: String,
}

impl ResponseStatus {
    pub fn fail(reason: String) -> ResponseStatus {
        ResponseStatus{ success: false, status_message: reason }
    }

    pub fn success() -> ResponseStatus {
        ResponseStatus{ success: true, status_message: "Success".into() }
    }
}

#[derive(Debug, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

pub use ResponseStatus as ValidateTokenResponse;

pub async fn validate_token(db_pool: &PgPool, token: &String) -> ValidateTokenResponse {
    let token_validation_query = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE user_token = $1")
        .bind(token)
        .fetch_optional(db_pool)
        .await;

    match token_validation_query {
        Ok(Some(_)) => ValidateTokenResponse::success(),
        Ok(None) => ValidateTokenResponse::fail("Token not validated".into()),
        Err(error) => {
            eprintln!("Error: Failed to validate token {}: {}", token, error);
            ValidateTokenResponse::fail("Token validation server internal error.".into())
        }
    }
}
