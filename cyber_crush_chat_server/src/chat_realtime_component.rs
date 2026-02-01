use shared_server_lib::common;

use axum::{
    extract::{State, ws::{WebSocket, WebSocketUpgrade, Message, CloseFrame}},
    response::IntoResponse,
};

use sqlx::{ PgPool, types::chrono::NaiveDateTime };
use serde::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use futures_util::stream::{ SplitSink, SplitStream };

use crate::common_chat::{ ServerState, ChatType };
use crate::common_chat;

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;
type SendingChannel = tokio::sync::mpsc::UnboundedSender<axum::extract::ws::Message>;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ChatClientMessage {
    Init { token: String, chat_id: i32 },
    Msg { token: String, message: String },
    Exit { token: String },
}

struct ConnectionData {
    token: String,
    user_id: i32,
    chat_id: i32,
    chat_type: ChatType,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ChatResponse {
    Info { text: String },
    Error { text: String },
    ChatMessage { chat_id: i32, sender: String, message: String, time_stamp: String },
}

pub async fn web_socket_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

pub async fn handle_socket(web_socket: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut receiver) = web_socket.split();

    let connection_data = match initialize_connection(&mut sender, &mut receiver, &state).await {
        Some(data) => data,
        None => { return; }
    };

    let (sending_channel, mut receiving_channel) = mpsc::unbounded_channel::<Message>();

    state.token_to_chat_id.insert(connection_data.token.clone(), connection_data.chat_id);
    state.chat_connections.entry(connection_data.chat_id).or_default().push((connection_data.user_id, sending_channel.clone()));

    let connection_success_response = ChatResponse::Info{ text: "user connection succeeded".into() };
    ws_send_chat_response(&mut sender, &connection_success_response).await;
    
    //Thread that sends messages from the channel to the websocket client
    tokio::spawn(async move {
        while let Some(msg) = receiving_channel.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        let client_message = match msg {
            Message::Text(text) => serde_json::from_str::<ChatClientMessage>(&text),
            Message::Close(_close_frame) => {
                break;
            },
            _ => {
                //TODO implement PING, PONG
                continue;
            }
        };

        match client_message {
            Ok(ChatClientMessage::Init{ .. }) => {
                let info_response = ChatResponse::Info{ text: "connection already initialized".into() };
                channel_send_chat_response(&sending_channel, &info_response);
            },
            Ok(ChatClientMessage::Msg{ token, message }) => {
                if token != connection_data.token {
                    eprintln!("Error: Token mismatch in message for user id: {}", connection_data.user_id);
                    break;
                }

                let time_stamp = chrono::Utc::now().naive_utc();

                if let Err(error) = update_database(&connection_data, &message, &time_stamp, &state.db_pool).await {
                    let error_response = ChatResponse::Error{ text: error };
                    channel_send_chat_response(&sending_channel, &error_response);
                    continue;
                }

                let sender_username_query = sqlx::query_scalar::<_, String>(
                r#"
                    SELECT username
                    FROM users
                    WHERE id = $1
                "#)
                .bind(connection_data.user_id)
                .fetch_optional(&state.db_pool)
                .await;

                let username = match sender_username_query {
                    Ok(Some(username)) => username,
                    Ok(None) => {
                        let error_response = ChatResponse::Error{ text: "sender username not found".to_string() };
                        channel_send_chat_response(&sending_channel, &error_response);
                        continue
                    },
                    Err(error) => {
                        let error_response = ChatResponse::Error{ text: "An error has occured while sending message through chat. Internal server error!".to_string() };
                        channel_send_chat_response(&sending_channel, &error_response);
                        eprintln!("Getting username for user id {} failed. Error: {}", connection_data.user_id, error);
                        continue
                    }
                };

                let chat_response = ChatResponse::ChatMessage {
                    chat_id: connection_data.chat_id,
                    sender: username,
                    message: message.clone(),
                    time_stamp: time_stamp.to_string(),
                };

                // Send message to all chat connected members
                if let Some(users) = state.chat_connections.get(&connection_data.chat_id) {
                    for (user_id, user_sender) in users.iter() {
                        if *user_id == connection_data.user_id {
                            continue;
                        }

                        channel_send_chat_response(&user_sender, &chat_response);
                    }
                }
            },
            Ok(ChatClientMessage::Exit{ token }) => {
                if connection_data.token == token {
                    break;
                }
            },
            Err(error) => {
                eprintln!("Error: Realtime chat component failed to receive chat client message, error: {}", error);
            }
        }
    }

    close_chat(connection_data, state);
}

async fn initialize_connection(sender: &mut WsSender, receiver: &mut WsReceiver, state: &ServerState) -> Option<ConnectionData> {
    match receiver.next().await
    {
        Some(Ok(Message::Text(text))) => {
            let (token, chat_id) = match serde_json::from_str::<ChatClientMessage>(&text) {
                Ok(ChatClientMessage::Init{ token, chat_id }) => ( token, chat_id ),
                Ok(_) => {
                    let error_response = ChatResponse::Error{ text: "Wrong client message type".into() };
                    ws_send_chat_response(sender, &error_response).await;
                    close_connection(sender).await;
                    return None;
                },
                Err(error) => {
                    eprintln!("Error: Realtime chat component failed while weceiving init message failed: {}", error);
                    let error_response = ChatResponse::Error{ text: "Internal connection request server error".into() };
                    ws_send_chat_response(sender, &error_response).await;
                    close_connection(sender).await;
                    return None;
                }
            };

            let (user_id, chat_type) = match validate_user_and_chat(state, &token, chat_id).await {
                Ok((id, chat_type)) => (id, chat_type),
                Err(error) => {
                    ws_send_chat_response(sender, &error).await;
                    close_connection(sender).await;
                    return None;
                }
            };

            return Some(ConnectionData{ token, user_id, chat_id, chat_type });
        }
        Some(_) => {
            let error_response = ChatResponse::Error{ text: "Wrong socket message type".into() };
            ws_send_chat_response(sender, &error_response).await;
            close_connection(sender).await;
            return None;
        },
        None => {
            let error_response = ChatResponse::Error{ text: "No message sent. Close connection".into() };
            ws_send_chat_response(sender, &error_response).await;
            close_connection(sender).await;
            return None;
        }
    };
}

async fn validate_user_and_chat(state: &ServerState, token: &String, chat_id: i32) -> Result<(i32, ChatType), ChatResponse> {
    let validated = common::validate_token(&state.db_pool, token).await;
    
    if validated.response_status.success == false {
        return Err(ChatResponse::Error{ text: "User not validated".into() })
    }

    let user_id = validated.id.unwrap();

    let chat_id_query = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM user_chats WHERE chat_id = $1 AND user_id = $2"
    )
    .bind(chat_id)
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await;

    match chat_id_query {
        Ok(Some(_id)) => (),
        Ok(None) => {
            return Err(ChatResponse::Error{ text: "User dones not belong to this chat".into() });
        }
        Err(error) => {
            eprintln!("Error: Realtime chat component failed while checking chat id for chat: {}, error: {}", chat_id, error);
            return Err(ChatResponse::Error{ text: "Internal validation server error 1".into() });
        }
    };

    let chat_type = match common_chat::get_chat_type(&state.db_pool, chat_id).await {
        Ok(chat_type) => chat_type,
        Err(error) => {
            return Err(ChatResponse::Error{ text: error });
        }
    };

    Ok((user_id, chat_type))
}

async fn update_database(connection_data: &ConnectionData, message: &String, time_stamp: &NaiveDateTime, db_pool :&PgPool) -> Result<(), String> {
    let mut transaction = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            eprintln!("Error: Realtime chat component failed to create transaction for user id: {} and chat id: {}, error: {}", connection_data.user_id, connection_data.chat_id, error);
            return Err("Failed to send message. Internal server error: 1".into());
        }
    };

    let insert_message_query = sqlx::query(
    r#"
        INSERT INTO chat_messages (chat_id, sender_id, content, time_stamp)
        VALUES ($1, $2, $3, $4)
    "#)
    .bind(connection_data.chat_id)
    .bind(connection_data.user_id)
    .bind(message.clone())
    .bind(time_stamp.clone())
    .execute(&mut *transaction)
    .await;

    if let Err(error) = insert_message_query {
        eprintln!("Error: Realtime chat component failed to insert message for user id: {} and chat id: {}, error: {}", connection_data.user_id, connection_data.chat_id, error);
        let _ = transaction.rollback().await;
        return Err("Failed to send message. Internal server error: 2".into());
    }
                    
    let chat_to_update = match connection_data.chat_type {
        ChatType::Direct => "direct_chats",
        ChatType::Group => "group_chats"
    };

    let update_last_metadata_sql = format!(
        r#"
            UPDATE {}
            SET last_message = $1, last_time_stamp = $2
            WHERE chat_id = $3
        "#,
        chat_to_update
    );

    let update_last_metadata_query = sqlx::query(&update_last_metadata_sql)
        .bind(message.clone())
        .bind(time_stamp.clone())
        .bind(connection_data.chat_id)
        .execute(&mut *transaction)
        .await;

    if let Err(error) = update_last_metadata_query {
        eprintln!("Error: Realtime chat component failed to update last metadata for user id: {} and chat id: {}, error: {}", connection_data.user_id, connection_data.chat_id, error);
        let _ = transaction.rollback().await;
        return Err("Failed to send message. Internal server error: 3".into());
    }

    if let Err(error) = transaction.commit().await {
        eprintln!("Error: Realtime chat component failed to commit transaction for user id: {} and chat id: {}, error: {}", connection_data.user_id, connection_data.chat_id, error);
        return Err("Failed to send message. Internal server error: 4".into());
    }

    Ok(())
}

async fn ws_send_chat_response(sender: &mut WsSender, chat_response: &ChatResponse) {
    let parsed_response = match serde_json::to_string(&chat_response) {
        Ok(json) => json.into(),
        Err(error) => {
            eprintln!("Error: Realtime chat component failed while parsing json for websocket: {}", error);
            return;
        }
    };

    if let Err(error) = sender.send(parsed_response).await {
        eprintln!("Error: Realtime chat component failed while sending using websocket: {}", error);
    }
}

fn channel_send_chat_response(sender: &SendingChannel, chat_response: &ChatResponse) {
    let parsed_response = match serde_json::to_string(&chat_response) {
        Ok(json) => json.into(),
        Err(error) => {
            eprintln!("Error: Realtime chat component failed while parsing json for channel response: {}", error);
            return;
        }
    };

    if let Err(error) = sender.send(Message::Text(parsed_response)) {
        eprintln!("Error: Realtime chat component failed while sending text using channel response: {}", error);
    }
}

async fn close_connection(sender: &mut WsSender) {
    if let Err(error) = sender.send(Message::Close(Some(CloseFrame{ code: axum::extract::ws::close_code::NORMAL, reason: "close".into() }))).await {
        eprintln!("Error: Realtime chat component failed while closeing a connection: {}", error);
        return;
    }

    if let Err(error) = sender.flush().await {
        eprintln!("Error: Realtime chat component failed while flushing close frame: {}", error);
        return;
    }
}

fn close_chat(connection_data: ConnectionData, state: Arc<ServerState>) {
    state.token_to_chat_id.remove(&connection_data.token);
    if let Some(mut vec) = state.chat_connections.get_mut(&connection_data.chat_id) {
        vec.retain(|(uid, _)| *uid != connection_data.user_id);
    }
}
