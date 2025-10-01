use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::PgPool;
use chrono;

#[derive(Debug, Deserialize)]
struct GetUserFundsRequest {
    token: String
}

#[derive(Debug, Serialize)]
struct GetUserFundsResponse {
    success: bool,
    message: String,
    funds: i32
}

#[derive(Debug, Deserialize)]
struct GetUserTransactionHistoryRequest {
    token: String
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct TransactionEntry {
    sending_username: String,
    receiving_username: String,
    message: String,
    transaction_amount: i32,
    time_stamp: chrono::NaiveDateTime
}

#[derive(Debug, Serialize)]
struct GetUserTransactionHistoryResponse {
    success: bool,
    message: String,
    transactions: Vec<TransactionEntry>
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

    let socket_addr = server_configuration.get_socket_addr(ServerType::Bank);

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Bank server running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(hello))
        .route("/get_user_funds", post(get_user_funds))
        .route("/get_user_transaction_history", post(get_user_transaction_history))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush bank server!"
}

async fn get_user_funds(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserFundsRequest>) -> impl IntoResponse   {
    #[derive(Debug, sqlx::FromRow)]
    struct FundsQuery {
        funds: i32,
    }

    // b - bank account
    // u - user
    let funds_query = sqlx::query_as::<_, FundsQuery>(
        r#"SELECT b.funds FROM bank_accounts b JOIN users u ON b.user_id = u.id WHERE u.user_token = $1"#)
        .bind(&payload.token)
        .fetch_optional(&state.db_pool)
        .await;

    let response = match funds_query {
        Ok(Some(funds_data)) => GetUserFundsResponse { success: true, message: "success".into(), funds: funds_data.funds },
        Ok(None) => GetUserFundsResponse { success: false, message: "No account found for this token".into(), funds: -1 },
        Err(error) => {
            eprintln!("Error: Getting user funds failed for token {} Error:{}", payload.token, error);
            GetUserFundsResponse { success: false, message: "No funds found. Server error!".into(), funds: -1 }
        }
    };

    Json(response) 
}


async fn get_user_transaction_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserTransactionHistoryRequest>) -> impl IntoResponse {
    let transactions_query = sqlx::query_as::<_, TransactionEntry>(r#"
        SELECT 
            sender.username AS sending_username,
            receiver.username AS receiving_username,
            t.message,
            t.transaction_amount,
            t.time_stamp
        FROM bank_transactions t
        JOIN bank_accounts sender_acc ON t.sending_id = sender_acc.id
        JOIN users sender ON sender_acc.user_id = sender.id
        JOIN bank_accounts receiver_acc ON t.receiving_id = receiver_acc.id
        JOIN users receiver ON receiver_acc.user_id = receiver.id
        WHERE sender.user_token = $1 OR receiver.user_token = $1
        ORDER BY t.time_stamp DESC;"#)
        .bind(&payload.token)
        .fetch_all(&state.db_pool)
        .await;
   
    let response = match transactions_query {
        Ok(transactions) => GetUserTransactionHistoryResponse { success: true, message: "success".into(), transactions },
        Err(error) => {
            eprintln!("Error: Getting user transaction history failed for token{} Error: {}", payload.token, error);
            GetUserTransactionHistoryResponse { success: false, message: "No transaction found. Server error!".into(), transactions: vec![] }
        }
    };

    Json(response)
}
/*
async fn transfer_funds() -> &'static str {
    "Transfer funds"
}
*/
