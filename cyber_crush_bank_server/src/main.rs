use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database, common::ResponseStatus};

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
struct GetUserFundsRequest {
    token: String
}

#[derive(Debug, Serialize)]
struct GetUserFundsResponse {
    response_status: ResponseStatus,
    funds: i32
}

#[derive(Debug, Deserialize)]
struct GetUserTransactionHistoryRequest {
    token: String
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct TransactionEntry {
    sender_username: String,
    receiver_username: String,
    message: String,
    amount: i32,
    time_stamp: chrono::NaiveDateTime
}

#[derive(Debug, Serialize)]
struct GetUserTransactionHistoryResponse {
    response_status: ResponseStatus,
    transactions: Vec<TransactionEntry>
}

#[derive(Debug, Deserialize)]
struct TransferFundsRequest {
    sender_token: String,
    receiver_username: String,
    message: String,
    amount: i32
}

use ResponseStatus as TransferFundsResponse;

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
        .route("/transfer_funds", post(transfer_funds))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush bank server!"
}

async fn get_user_funds(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserFundsRequest>) -> impl IntoResponse   {
    let funds_query : Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
    r#"
        SELECT b.funds FROM bank_accounts b
        JOIN users u ON b.user_id = u.id
        WHERE u.user_token = $1
    "#)
    .bind(&payload.token)
    .fetch_optional(&state.db_pool)
    .await;

    let response = match funds_query {
        Ok(Some(funds)) => GetUserFundsResponse{ response_status: ResponseStatus::success(), funds },
        Ok(None) => GetUserFundsResponse{ response_status: ResponseStatus::fail("No account found for this token".into()), funds: -1 },
        Err(error) => {
            eprintln!("Error: Getting user funds failed for token {} Error:{}", payload.token, error);
            GetUserFundsResponse{ response_status: ResponseStatus::fail("No funds found. Server error!".into()), funds: -1 }
        }
    };

    Json(response) 
}

async fn get_user_transaction_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserTransactionHistoryRequest>) -> impl IntoResponse {
    let transactions_query = sqlx::query_as::<_, TransactionEntry>(
    r#"
        SELECT 
            sender.username AS sender_username,
            receiver.username AS receiver_username,
            t.message,
            t.amount,
            t.time_stamp
        FROM bank_transactions t
        JOIN bank_accounts sender_acc ON t.sender_id = sender_acc.id
        JOIN users sender ON sender_acc.user_id = sender.id
        JOIN bank_accounts receiver_acc ON t.receiver_id = receiver_acc.id
        JOIN users receiver ON receiver_acc.user_id = receiver.id
        WHERE sender.user_token = $1 OR receiver.user_token = $1
        ORDER BY t.time_stamp DESC;
    "#)
    .bind(&payload.token)
    .fetch_all(&state.db_pool)
    .await;
   
    let response = match transactions_query {
        Ok(transactions) => GetUserTransactionHistoryResponse{ response_status: ResponseStatus::success(), transactions },
        Err(error) => {
            eprintln!("Error: Getting user transaction history failed for token {} Error: {}", payload.token, error);
            GetUserTransactionHistoryResponse{ response_status: ResponseStatus::fail("No transaction found. Server error!".into()), transactions: vec![] }
        }
    };

    Json(response)
}

async fn transfer_funds(State(state): State<Arc<ServerState>>, Json(payload): Json<TransferFundsRequest>) -> impl IntoResponse {
    #[derive(Debug, sqlx::FromRow)]
    struct BankAccount {
        id: i32
    }
    
    // Retreive sender account by token
    let sender_account_query = sqlx::query_as::<_, BankAccount>(
     r#"
        SELECT 
            b.id
        FROM bank_accounts b
        JOIN users u ON b.user_id = u.id
        WHERE u.user_token = $1
    "#
    )
    .bind(&payload.sender_token)
    .fetch_optional(&state.db_pool)
    .await;
    
    let sender_account = match sender_account_query {
        Ok(Some(account)) => account,
        Ok(None) => {
            return Json(TransferFundsResponse::fail("No bank account found".into()));
        },
        Err(error) => {
            eprintln!("Error: Transfering funds failed while getting sender account {}, Error: {}", payload.sender_token, error);
            return Json(TransferFundsResponse::fail("No bank account found(sender). Server Error!".into()));
        }
    };
    
    // Retrieve receiver account by username
    let receiver_account_query = sqlx::query_as::<_, BankAccount>(
    r#"
        SELECT 
            b.id
        FROM bank_accounts b
        JOIN users u ON b.user_id = u.id
        WHERE u.username = $1
    "#
    )
    .bind(&payload.receiver_username)
    .fetch_optional(&state.db_pool)
    .await;

    let receiver_account = match receiver_account_query {
        Ok(Some(account)) => account,
        Ok(None) => {
            return Json(TransferFundsResponse::fail("Receiver not found".into()));
        },
        Err(error) => {
            eprintln!("Error: Transfering funds failed while getting receiver account {}, Error: {}", payload.receiver_username, error);
            return Json(TransferFundsResponse::fail("No bank account found(receiver). Server Error!".into()));
        }
    };

    let mut transaction: Transaction<'_, Postgres> = match state.db_pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            eprintln!("Error: Transfering funds failed while starting the transaction. Error: {}", error);
            return Json(TransferFundsResponse::fail("Internal server Error: 1!".into()));
        }
    };

    let subtract_funds_query = sqlx::query(
    r#"
        UPDATE bank_accounts
        SET funds = funds - $1
        WHERE id = $2 AND funds > $1
    "#)
    .bind(payload.amount)
    .bind(sender_account.id)
    .execute(&mut *transaction)
    .await;
    
    match subtract_funds_query {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return Json(TransferFundsResponse::fail("Not enouch funds".into()));
            }
            if result.rows_affected() != 1 {
                eprintln!("Error: Transfering funds failed too may rows affected while subtracting funds!");
                return Json(TransferFundsResponse::fail("Internal server Error: 2!".into()));
            }
        }
        Err(error) => {
            eprintln!("Error: Transfering funds failed while subtracting funds. Error: {}", error);
            return Json(TransferFundsResponse::fail("Internal server Error: 3!".into()));
        }
    };

    let add_funds_query = sqlx::query(
    r#"
        UPDATE bank_accounts
        SET funds = funds + $1
        WHERE id = $2
    "#)
    .bind(payload.amount)
    .bind(receiver_account.id)
    .execute(&mut *transaction)
    .await;

    match add_funds_query {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return Json(TransferFundsResponse::fail("Receiver not found".into()));
            }
            if result.rows_affected() != 1 {
                eprintln!("Error: Transfering funds failed too may rows affected while adding funds!");
                return Json(TransferFundsResponse::fail("Internal server Error: 4!".into()));
            }
        }
        Err(error) => {
            eprintln!("Error: Transfering funds failed while adding funds. Error: {}", error);
            return Json(TransferFundsResponse::fail("Internal server Error: 5!".into()));
        }    
    };

    let create_transaction_query = sqlx::query(
    r#"
        INSERT INTO bank_transactions
        (sender_id, receiver_id, message, amount, time_stamp)
        VALUES ($1, $2, $3, $4, NOW())
    "#)
    .bind(sender_account.id)
    .bind(receiver_account.id)
    .bind(payload.message)
    .bind(payload.amount)
    .execute(&mut *transaction)
    .await;

    match create_transaction_query {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return Json(TransferFundsResponse::fail("Receiver not found".into()));
            }
            if result.rows_affected() != 1 {
                eprintln!("Error: Transfering funds failed too may rows affected while inserting transaction!");
                return Json(TransferFundsResponse::fail("Internal server Error: 6!".into()));
            }
        }
        Err(error) => {
            eprintln!("Error: Transfering funds failed while inserting transaction. Error: {}", error);
            return Json(TransferFundsResponse::fail("Internal server Error: 7!".into()));
        }    
    };

    let commited_transaction = transaction.commit().await;

    if commited_transaction.is_err() {
       eprintln!("Error: Transfering funds failed while commiting transaction. Error: {}", commited_transaction.unwrap_err());
        return Json(TransferFundsResponse::fail("Internal server Error: 8!".into()));
    }

    Json(TransferFundsResponse::success())
}
