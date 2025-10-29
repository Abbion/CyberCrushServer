use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database};

use axum::{
    routing::{get, post},
    Router,
};

use tokio::net::TcpListener;
use std::sync::Arc;
use dashmap::DashMap;

mod common_chat;
mod chat_request_component;
mod chat_realtime_component;

use crate::common_chat::ServerState;

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_posgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ db_pool, token_to_chat_id: Arc::new(DashMap::new()), chat_connections: Arc::new(DashMap::new()) });

    let socket_addr = server_configuration.get_socket_addr(ServerType::Chat);

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Chat server running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(chat_request_component::hello))
        .route("/get_user_chats", post(chat_request_component::get_user_chats))
        .route("/get_chat_history", post(chat_request_component::get_chat_history))
        .route("/get_chat_metadata", post(chat_request_component::get_chat_metadata))
        .route("/create_new_direct_chat", post(chat_request_component::create_new_direct_chat))
        .route("/web_socket", get(chat_realtime_component::web_socket_handler))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}
