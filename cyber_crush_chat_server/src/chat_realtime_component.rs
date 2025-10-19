use axum::{
    extract::{State, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::common_chat::ServerState;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ChatChlientMessage {
    Init { token: String, chat_id: i32 },
    Msg { token: String, message: String },
    Exit { token: String },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ChatResponse {
    Info { text: String },
    Error { text: String },
    ChatMessage { chat_id: i32, message: String, time_stamp: String },
}


pub async fn web_socket_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

pub async fn handle_socket(web_socket: WebSocket, state: Arc<ServerState>){
    let (mut sender, mut receiver) = web_socket.split();

    if let Some(Ok(Message::Text(text))) = receiver.next().await {
        if let Ok(ChatChlientMessage::Init{ token, chat_id }) = serde_json::from_str::<ChatChlientMessage>(&text) {
            match validate_user_and_chat(&state, &token, chat_id).await {
                Ok(user_id) => {
                    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
                    state.token_to_chat_id.insert(token.clone(), chat_id);
                    state.chat_connections.entry(chat_id).or_default().push((user_id, tx.clone()));

                    let msg = ChatResponse::Info{ text: format!("Nice to meed you: {}", token) };
                    let _ = sender.send(Message::Text(serde_json::to_string(&msg).unwrap().into())).await;

                    let mut ws_sender = sender;
                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await{
                            if ws_sender.send(msg).await.is_err() {
                                break;
                            }
                        }
                    });
                }
                Err(error) => {
                    let _ = sender.send(Message::Text(serde_json::to_string(&ChatResponse::Error{ text: error }).unwrap().into())).await;
                }
            }
        }
        else {
            let _ = sender.send(Message::Text(serde_json::to_string(&ChatResponse::Error { text: "Invalid init message".into() }).unwrap().into())).await;
        }
    }

    println!("END socket");
}

async fn validate_user_and_chat(state: &ServerState, token: &str, chat_id: i32) -> Result<i32, String> {
    let user_id_query = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM users WHERE user_token = $1"
    )
    .bind(token)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let user_id = user_id_query.ok_or("Invalid token")?;

    let chat_check = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM direct_chats WHERE id = $1 AND (user_a_id = $2 OR user_b_id = $2)"
    )
    .bind(chat_id)
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    if chat_check.is_none() {
        return Err("Chat not found or user not participant".into());
    }

    Ok(user_id)
}
