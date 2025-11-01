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

#[derive(Debug)]
pub struct ValidateTokenResponse {
    pub response_status: ResponseStatus,
    pub id : Option<i32>
}

use ValidateTokenResponse as UserIdByUsernameResponse;

pub async fn validate_token(db_pool: &PgPool, token: &String) -> ValidateTokenResponse {
    let token_validation_query = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE user_token = $1")
        .bind(token)
        .fetch_optional(db_pool)
        .await;

    match token_validation_query {
        Ok(Some(id)) => ValidateTokenResponse{ response_status: ResponseStatus::success(), id: Some(id) },
        Ok(None) => ValidateTokenResponse{ response_status: ResponseStatus::fail("Token not validated".into()), id: None },
        Err(error) => {
            eprintln!("Error: Failed to validate token {}: {}", token, error);
            ValidateTokenResponse{ response_status: ResponseStatus::fail("Token validation server internal error.".into()), id: None }
        }
    }
}

pub async fn get_user_id_by_username(db_pool: &PgPool, username: &String) -> UserIdByUsernameResponse {
    let user_id_query = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(db_pool)
        .await;

    match user_id_query {
        Ok(Some(id)) => UserIdByUsernameResponse{ response_status: ResponseStatus::success(), id: Some(id) },
        Ok(None) => UserIdByUsernameResponse{ response_status: ResponseStatus::fail("User not found".into()), id: None },
        Err(error) => {
            eprintln!("Error Failed to get user id by username: {}, Error: {}", username, error);
            UserIdByUsernameResponse{ response_status: ResponseStatus::fail("User not found internal server error".into()), id: None }
        }
    }
}
