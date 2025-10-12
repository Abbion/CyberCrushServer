use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct ValidateTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateTokenResponse {
    pub success: bool,
    message: String
}

pub async fn validate_token(db_pool: &PgPool, token: &String) -> ValidateTokenResponse {
    let token_validation_query = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE user_token = $1")
        .bind(token)
        .fetch_optional(db_pool)
        .await;

    match token_validation_query {
        Ok(Some(_)) => {
                ValidateTokenResponse{ success: true, message: "success".into() }
        },
        Ok(None) => {
            ValidateTokenResponse{ success: false, message: "Token not validated".into() }
        },
        Err(error) => {
            eprintln!("Error: Failed to validate token {}: {}", token, error);
            ValidateTokenResponse{ success: false, message: "Token validation server internal error.".into() }
        }
    }
}
