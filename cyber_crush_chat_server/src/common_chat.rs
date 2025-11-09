use dashmap::DashMap;
use tokio::sync::mpsc;
use axum::extract::ws::Message;
use sqlx::PgPool;
use std::sync::Arc;

pub enum ChatType {
    Direct,
    Group,
}

pub struct ServerState {
    pub db_pool: PgPool, //This is thread safe
    pub token_to_chat_id: Arc<DashMap<String, i32>>,
    pub chat_connections: Arc<DashMap<i32, Vec<(i32, mpsc::UnboundedSender<Message>)>>>, //chat_id -> Vec<(user_id, sending stream)>
}

pub async fn get_chat_type(db_pool: &PgPool, chat_id: i32) -> Result<ChatType, String> {
    let chat_type_query = sqlx::query_scalar::<_, i32>(
    r#"
        SELECT
            CASE
                WHEN EXISTS (SELECT 1 FROM direct_chats WHERE chat_id = $1) THEN 1
                WHEN EXISTS (SELECT 1 FROM group_chats WHERE chat_id = $1) THEN 2
                ELSE 0
            END AS chat_type
    "#)
    .bind(chat_id)
    .fetch_one(db_pool)
    .await;

    match chat_type_query {
        Ok(1) => {
            Ok(ChatType::Direct)
        },
        Ok(2) => {
            Ok(ChatType::Group)
        }
        Ok(_) => {
            Err(String::from("Unknown chat type"))
        }
        Err(error) => {
            eprintln!("Error: Getting chat type failed for chat id: {}, Error: {}", chat_id, error);
            Err(String::from("Internal chat identification server error"))
        }
    }
}
