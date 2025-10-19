use shared_server_lib::{common, common::ResponseStatus};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono;
use std::cmp;

use crate::common_chat::ServerState;

#[derive(Debug, Deserialize)]
pub struct GetUserChatsRequest {
    token: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DirectChat {
    chat_id: i32,
    chat_partner: String,
    last_message: Option<String>,
    last_message_time_stamp: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct GetUserChatsResponse {
    response_status: ResponseStatus,
    chats: Option<Vec<DirectChat>>,
}

#[derive(Debug, Deserialize)]
pub struct GetDirectChatHistoryRequest {
    token: String,
    chat_id: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChatMessage {
    sender: String,
    message: String,
    time_stamp: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct GetDirectChatHistoryResponse {
    response_status: ResponseStatus,
    username_a: String,
    username_b: String,
    messages: Vec<ChatMessage>,
}

impl GetDirectChatHistoryResponse {
    fn fail(reason: &str) -> GetDirectChatHistoryResponse {
        GetDirectChatHistoryResponse{
            response_status: ResponseStatus::fail(reason.into()),
            username_a: "".into(),
            username_b: "".into(),
            messages: vec![] }
    }

    fn success(username_a: String, username_b: String, messages: Vec<ChatMessage>) -> GetDirectChatHistoryResponse {
        GetDirectChatHistoryResponse{
            response_status: ResponseStatus::success(),
            username_a,
            username_b,
            messages }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateNewDirectChatRequest {
    token: String,
    receiver_username: String,
}

#[derive(Debug, Serialize)]
pub struct CreateNewDirectChatResponse {
    response_status: ResponseStatus,
    chat_id: i32,
}

impl CreateNewDirectChatResponse {
    fn fail(reason: &str) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::fail(reason.into()),
            chat_id: -1 }
    }

    fn success(chat_id: i32) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::success(),
            chat_id }
    }

    fn chat_exists(chat_id: i32) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::fail("Direct chat already exsits!".into()),
            chat_id }
    }
}

pub async fn hello() -> &'static str {
    "Hello, cyber crush chat server!"
}

pub async fn get_user_chats(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserChatsRequest>) -> impl IntoResponse {
    let direct_chats_query = sqlx::query_as::<_, DirectChat>(
    r#"
        SELECT 
            dc.id AS chat_id,
            CASE
                WHEN dc.user_a_id = u.id THEN u2.username
                ELSE u1.username
            END AS chat_partner,
            dc.last_message,
            dc.last_message_time_stamp
        FROM direct_chats dc
        JOIN users u1 ON dc.user_a_id = u1.id
        JOIN users u2 ON dc.user_b_id = u2.id
        JOIN users u ON u.user_token = $1
        WHERE dc.user_a_id = u.id OR dc.user_b_id = u.id
        ORDER BY dc.last_message_time_stamp DESC;
    "#)
    .bind(&payload.token)
    .fetch_all(&state.db_pool)
    .await;

    let direct_chats = match direct_chats_query {
        Ok(chats) => chats,
        Err(error) => {
            eprintln!("Error: Getting user direct chats failed for token {}, Error: {}", payload.token, error);
            return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("Internal server error 1".into()), chats: None });
        }
    };

    Json(GetUserChatsResponse{ response_status: ResponseStatus::success(), chats: Some(direct_chats) })
}


pub async fn get_direct_chat_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetDirectChatHistoryRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.success == false {
        return Json(GetDirectChatHistoryResponse::fail("Token validation failed".into()));
    }

    #[derive(Debug, sqlx::FromRow)]
    struct ChatParticipantInfo {
        user_a_username: String,
        user_b_username: String,
    }

    let participant_info_query = sqlx::query_as::<_, ChatParticipantInfo>(
    r#"
        SELECT
            ua.username AS user_a_username,
            ub.username AS user_b_username
        FROM direct_chats dc
        JOIN users ua ON dc.user_a_id = ua.id
        JOIN users ub ON dc.user_b_id = ub.id
        WHERE dc.id = $1
    "#)
    .bind(payload.chat_id)
    .fetch_optional(&state.db_pool)
    .await;

    let participant_info = match participant_info_query {
        Ok(Some(info)) => info,
        Ok(None) => {
            return Json(GetDirectChatHistoryResponse::fail("Chat not found".into()));
        }
        Err(error) => {
            eprintln!("Error: Getting user chat history failed 1 for token: {}, Error: {}", payload.token, error);
            return Json(GetDirectChatHistoryResponse::fail("Internal server error: 1".into()));
        }
    };

    let chat_messages_query = sqlx::query_as::<_, ChatMessage>(
    r#"
        SELECT
            u.username AS sender,
            cm.message,
            cm.time_stamp
        FROM direct_chat_messages cm
        JOIN users u ON cm.sender_id = u.id
        WHERE cm.chat_id = $1
        ORDER BY cm.time_stamp DESC
        LIMIT 50
    "#)
    .bind(payload.chat_id)
    .fetch_all(&state.db_pool)
    .await;
    
    let chat_messages = match chat_messages_query {
        Ok(mut messages) => { 
            messages.reverse();
            messages
        }
        Err(error) => {
            eprintln!("Error: Getting user chat history failed 2 for token: {}, Error: {}", payload.token, error);
            return Json(GetDirectChatHistoryResponse::fail("Internal server error: 2".into()));
        }
    };

    Json(GetDirectChatHistoryResponse::success(
            participant_info.user_a_username,
            participant_info.user_b_username, 
            chat_messages))
}

pub async fn create_new_direct_chat(State(state): State<Arc<ServerState>>, Json(payload): Json<CreateNewDirectChatRequest>) -> impl IntoResponse {
    let sender_id_query = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM users WHERE user_token = $1"
    )
    .bind(&payload.token)
    .fetch_optional(&state.db_pool)
    .await;

    let sender_id = match sender_id_query {
        Ok(Some(id)) => id,
        Ok(None) => {
            return Json(CreateNewDirectChatResponse::fail("User not validated!"));
        },
        Err(error) => {
            eprintln!("Error: Creating direct chat failed while getting the sender id for token: {} and receiver: {}, Error: {}",
                      payload.token, payload.receiver_username, error);
            return Json(CreateNewDirectChatResponse::fail("Internal server error 1"));
        }
    };

    let receiver_id_query = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM users WHERE username = $1"
    )
    .bind(&payload.receiver_username)
    .fetch_optional(&state.db_pool)
    .await;

    let receiver_id = match receiver_id_query {
        Ok(Some(id)) => id,
        Ok(None) => {
            return Json(CreateNewDirectChatResponse::fail("Receiver not found!"));
        },
        Err(error) => {
            eprintln!("Error: Creating direct chat failed while getting the receiver id for token: {} and receiver: {}, Error: {}",
                      payload.token, payload.receiver_username, error);
            return Json(CreateNewDirectChatResponse::fail("Internal server error 2"));
        }
    };

    let min_id = cmp::min(sender_id, receiver_id);
    let max_id = cmp::max(sender_id, receiver_id);

    let find_direct_chat_query = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM direct_chats WHERE user_a_id = $1 AND user_b_id = $2"
    )
    .bind(&min_id)
    .bind(&max_id)
    .fetch_optional(&state.db_pool)
    .await;

    match find_direct_chat_query {
        Ok(Some(id)) => {
            return Json(CreateNewDirectChatResponse::chat_exists(id));
        },
        Ok(None) => (),
        Err(error) => {
            eprintln!("Error: Creating direct chat failed while checking if direct chat exists for token: {} and sender: {}, Error: {}", 
                      payload.token, payload.receiver_username, error);
            return Json(CreateNewDirectChatResponse::fail("Internal server error 3"));
        }
    };

    // Info: On conflict we do a simple do update, that does nothing.
    // This enables the query to return chat id
    // If 'DO UPDATE SET' would be replaced by 'DO NOTHING', the query would return NULL
    let create_new_direct_chat_query = sqlx::query_scalar::<_, i32>(
    r#"
        INSERT INTO direct_chats (user_a_id, user_b_id, last_message, last_message_time_stamp)
        VALUES ($1, $2, NULL, NULL)
        ON CONFLICT (user_a_id, user_b_id) DO UPDATE SET user_a_id = EXCLUDED.user_a_id
        RETURNING id
    "#)
    .bind(&min_id)
    .bind(&max_id)
    .fetch_one(&state.db_pool)
    .await;

    let response = match create_new_direct_chat_query {
        Ok(id) => CreateNewDirectChatResponse::success(id),
        Err(error) => {
            eprintln!("Error: Creating direct chat failed while inserting a new direct chat for token: {} and sender: {}, Error: {}", 
                      payload.token, payload.receiver_username, error);
            CreateNewDirectChatResponse::fail("Internal server error 4")
        }
    };

    Json(response)
}
