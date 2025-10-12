use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database, common_requests};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::{PgPool, Postgres, Transaction};
use chrono;

#[derive(Debug, Deserialize)]
struct GetUserChatsRequest {
    token: String,
}

//TODO: Change the database restrication NOT NULL on last message and last message time stamp
#[derive(Debug, Serialize, sqlx::FromRow)]
struct DirectChat {
    chat_id: i32,
    chat_partner: String,
    last_message: Option<String>,
    last_message_time_stamp: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
struct GetUserChatsResponse {
    success: bool,
    message: String,
    chats: Option<Vec<DirectChat>>,
}

#[derive(Debug, Deserialize)]
struct GetDirectChatHistoryRequest {
    token: String,
    chat_id: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct ChatMessage {
    sender: String,
    message: String,
    time_stamp: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct GetDirectChatHistoryResponse {
    success: bool,
    message: String,
    username_a: String,
    username_b: String,
    messages: Vec<ChatMessage>,
}

impl GetDirectChatHistoryResponse {
        fn fail(reason: &str) -> GetDirectChatHistoryResponse {
        GetDirectChatHistoryResponse{ 
            success: false,
            message: reason.into(),
            username_a: "".into(),
            username_b: "".into(),
            messages: vec![] }
    }

    fn success(username_a: String, username_b: String, messages: Vec<ChatMessage>) -> GetDirectChatHistoryResponse {
        GetDirectChatHistoryResponse{ 
            success: true,
            message: "success".into(),
            username_a,
            username_b,
            messages }
    }
}

#[derive(Debug)]
struct ServerState {
    db_pool: PgPool, //This is thread safe
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_posgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ db_pool });

    let socket_addr = server_configuration.get_socket_addr(ServerType::Chat);

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Chat server running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(hello))
        .route("/get_user_chats", post(get_user_chats))
        .route("/get_direct_chat_history", post(get_direct_chat_history))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush chat server!"
}

async fn get_user_chats(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserChatsRequest>) -> impl IntoResponse {
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
            return Json(GetUserChatsResponse{ success: false, message: "Internal server error 1".into(), chats: None });
        }
    };

    Json(GetUserChatsResponse{ success: true, message: "success".into(), chats: Some(direct_chats) })
}


async fn get_direct_chat_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetDirectChatHistoryRequest>) -> impl IntoResponse {
    let validated = common_requests::validate_token(&state.db_pool, &payload.token).await;
    
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
