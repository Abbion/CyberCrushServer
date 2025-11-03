use shared_server_lib::common;

use axum::{
    extract::{State, ws::{WebSocket, WebSocketUpgrade, Message, CloseFrame}},
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use futures_util::stream::{ SplitSink, SplitStream };

use crate::common_chat::{ ServerState, ChatType };
use crate::common_chat;

type WsSender = SplitSink<WebSocket, Message>;
type WsReceiver = SplitStream<WebSocket>;

use std::any::type_name;
fn print_type_of<T>(_:&T) {
    println!("{}", type_name::<T>());
}

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
    ChatMessage { chat_id: i32, message: String, time_stamp: String },
}


pub async fn web_socket_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

pub async fn handle_socket(web_socket: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut receiver) = web_socket.split();

    let connection_data = match initialize_connection(&mut sender, &mut receiver, &state).await {
        Some(data) => data,
        None => { 
            return;
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    state.token_to_chat_id.insert(connection_data.token.clone(), connection_data.chat_id);
    state.chat_connections.entry(connection_data.chat_id).or_default().push((connection_data.user_id, tx.clone()));

    let connection_success_response = ChatResponse::Info{ text: format!("User connected to chat!") };
    send_chat_response(&mut sender, &connection_success_response).await;
    
    //Thread that sends messages from the channel to the websocket client
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ChatClientMessage>(&text) {
                Ok(ChatClientMessage::Msg { token, message }) => {
                    if token != connection_data.token {
                        eprintln!("Error: token mismatch in message");
                        continue;
                    }

                    let time_stamp = chrono::Utc::now().to_rfc3339();
                    let chat_response = ChatResponse::ChatMessage {
                        chat_id: connection_data.chat_id,
                        message: message.clone(),
                        time_stamp: time_stamp.clone(),
                    };

                    // Wysyłamy do wszystkich użytkowników w czacie
                    if let Some(users) = state.chat_connections.get(&connection_data.chat_id) {
                        for (user_id, tx) in users.iter() {
                            if *user_id != connection_data.user_id {
                                if let Ok(json_text) = serde_json::to_string(&chat_response) {
                                    let _ = tx.send(Message::Text(json_text.into()));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    state.token_to_chat_id.remove(&connection_data.token);
    if let Some(mut vec) = state.chat_connections.get_mut(&connection_data.chat_id) {
        vec.retain(|(uid, _)| *uid != connection_data.user_id);
    }

    println!("END socket");
}

async fn initialize_connection(sender: &mut WsSender, receiver: &mut WsReceiver, state: &ServerState) -> Option<ConnectionData> {
    match receiver.next().await
    {
        Some(Ok(Message::Text(text))) => {
            let (token, chat_id) = match serde_json::from_str::<ChatClientMessage>(&text) {
                Ok(ChatClientMessage::Init{ token, chat_id }) => ( token, chat_id ),
                Ok(_) => {
                    let error_response = ChatResponse::Error{ text: "Wrong client message type".into() };
                    send_chat_response(sender, &error_response).await;
                    close_connection(sender).await;
                    return None;
                },
                Err(error) => {
                    eprintln!("Error: receiving init message failed: {}", error);
                    let error_response = ChatResponse::Error{ text: "Internal connection request server error".into() };
                    send_chat_response(sender, &error_response).await;
                    close_connection(sender).await;
                    return None;
                }
            };

            let (user_id, chat_type) = match validate_user_and_chat(state, &token, chat_id).await {
                Ok((id, chat_type)) => (id, chat_type),
                Err(error) => {
                    send_chat_response(sender, &error).await;
                    close_connection(sender).await;
                    return None;
                }
            };

            return Some(ConnectionData{ token, user_id, chat_id, chat_type });
        }
        Some(_) => {
            let error_response = ChatResponse::Error{ text: "Wrong socket message type".into() };
            send_chat_response(sender, &error_response).await;
            close_connection(sender).await;
            return None;
        },
        None => {
            let error_response = ChatResponse::Error{ text: "No message sent. Close connection".into() };
            send_chat_response(sender, &error_response).await;
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
            eprintln!("Error: realtime chat component failed while checking chat id for chat: {}, error: {}", chat_id, error);
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

async fn send_chat_response(sender: &mut WsSender, chat_response: &ChatResponse) {
    let parsed_response = match serde_json::to_string(&chat_response) {
        Ok(parsed_str) => parsed_str.into(),
        Err(error) => {
            eprintln!("Error: realtime chat component failed while parsing chat response: {:?}, Error: {}", chat_response, error);
            return;
        }
    };

    match sender.send(parsed_response).await {
        Ok(_) => (),
        Err(error) => {
            eprintln!("Error: realtime chat component failed while sending chat response: {:?}, error: {}", chat_response, error);
        }
    }
}

async fn close_connection(sender: &mut WsSender) {
    if let Err(error) = sender.send(Message::Close(Some(CloseFrame{ code: axum::extract::ws::close_code::NORMAL, reason: "close".into() }))).await {
        eprintln!("Error: realtime chat component failed while closeing a connection: {}", error);
        return;
    }

    if let Err(error) = sender.flush().await {
        eprintln!("Error: realtime chat component failed while flushing close frame: {}", error);
        return;
    }
    
    //TODO Clean the state

    println!("Close connection!");
}
