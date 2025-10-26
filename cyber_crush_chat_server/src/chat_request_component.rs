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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GroupChat{
    chat_id: i32,
    title: String,
    last_message: Option<String>,
    last_message_time_stamp: Option<chrono::NaiveDateTime>
}

#[derive(Debug, Serialize)]
pub struct GetUserChatsResponse {
    response_status: ResponseStatus,
    direct_chats: Option<Vec<DirectChat>>,
    group_chats: Option<Vec<GroupChat>>
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
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("User not validated".into()), direct_chats: None, group_chats: None });
    }

    let user_id = validated.id;
    
    let direct_chats_query = sqlx::query_as::<_, DirectChat>(
    r#"
        SELECT dc.chat_id, u.username AS chat_partner, dc.last_message, dc.last_time_stamp AS last_message_time_stamp
        FROM direct_chats dc
        JOIN user_chats uc1 ON uc1.chat_id = dc.chat_id
        JOIN user_chats uc2 ON uc2.chat_id = dc.chat_id
        JOIN users u ON u.id = uc2.user_id
        WHERE uc1.user_id = $1 AND uc2.user_id != $1;
    "#)
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    let direct_chats = match direct_chats_query {
        Ok(chats) => chats,
        Err(error) => {
            eprintln!("Error: Getting user chats failed while querying direct chats for token: {}, Error: {}", payload.token, error);
            return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("Internal server error: 1".into()), direct_chats: None, group_chats: None });
        }
    };

    let group_chats_query = sqlx::query_as::<_, GroupChat>(
    r#"
        SELECT gc.chat_id, gc.title AS title, gc.last_message, gc.last_time_stamp AS last_message_time_stamp
        FROM group_chats gc
        JOIN user_chats uc ON uc.chat_id = gc.chat_id
        WHERE uc.user_id = $1;
    "#)
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    let group_chats = match group_chats_query {
        Ok(chats) => chats,
        Err(error) => {
            eprintln!("Error: Getting user chats failed while querying group chats for token: {}, Error: {}", payload.token, error);
            return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("Internal server error: 2".into()), direct_chats: None, group_chats: None });
        }
    };

    Json(GetUserChatsResponse{ response_status: ResponseStatus::success(), direct_chats: Some(direct_chats), group_chats: Some(group_chats) })
}

pub async fn get_direct_chat_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetDirectChatHistoryRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
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
