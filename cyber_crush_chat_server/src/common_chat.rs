use dashmap::DashMap;
use tokio::sync::mpsc;
use axum::extract::ws::Message;
use sqlx::PgPool;
use std::sync::Arc;

pub struct ServerState {
    pub db_pool: PgPool, //This is thread safe
    pub token_to_chat_id: Arc<DashMap<String, i32>>,
    pub chat_connections: Arc<DashMap<i32, Vec<(i32, mpsc::UnboundedSender<Message>)>>>, //chat_id -> Vec<(user_id, sending stream)>
}
