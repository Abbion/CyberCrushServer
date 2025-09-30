use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
struct GetUserDataRequest {
    token: String,
}

#[derive(Debug, Serialize)]
struct GetUserDataResponse {
    success: bool,
    message: String,
    username: String,
    personal_number: String,
    extra_data: String,
}

#[derive(Debug, Serialize)]
struct GetAllUsernamesResponse {
    success: bool,
    message: String,
    usernames: Vec<String>
}

impl GetUserDataResponse {
    fn fail(reason: &str) -> GetUserDataResponse {
        GetUserDataResponse{ success: false, message: reason.into(), username: "".into(), personal_number: "".into(), extra_data: "".into() }
    }

    fn success(username: String, personal_number: String, extra_data: String) -> GetUserDataResponse {
        GetUserDataResponse{ success: true, message: "success".into(), username, personal_number, extra_data }
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
    
    let socket_addr = server_configuration.get_socket_addr(ServerType::Data);
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Data server running at: {}", socket_addr);

    let app = Router::new()
        .route("/", get(hello))
        .route("/get_all_usernames", get(get_all_usernames))
        .route("/get_user_data", post(get_user_data))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush data server!"
}

async fn get_user_data(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserDataRequest>) -> impl IntoResponse {
    #[derive(Debug, sqlx::FromRow)]
    struct UserDataQuery {
        id: i32,
        username: String,
        personal_number: i32,
        extra_data: serde_json::Value,
    }

    let user_data_query = sqlx::query_as::<_, UserDataQuery>(
        r#"SELECT id, username, personal_number, extra_data FROM users WHERE user_token = $1"#)
        .bind(&payload.token)
        .fetch_optional(&state.db_pool)
        .await;
    
    let response = match user_data_query {
        Ok(user_data) => {
            match user_data {
                Some(raw_data) => GetUserDataResponse::success(raw_data.username, raw_data.personal_number.to_string(), raw_data.extra_data.to_string()),
                None => GetUserDataResponse::fail("No user data found.")
            }
        },
        Err(error) => {
            eprintln!("Error: Getting user data failed for token: {}. Error: {}", payload.token, error);
            GetUserDataResponse::fail("No user data found. Server error!")
        }
    };

    Json(response)
}

async fn get_all_usernames(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let rows: Result<Vec<(String,)>, sqlx::Error> = sqlx::query_as(
        r#"SELECT username FROM users"#)
        .fetch_all(&state.db_pool)
        .await;

    let response = match rows {
        Ok(user_rows) => {
            let usernames: Vec<String> = user_rows.into_iter().map(|(u,)| u).collect();
            GetAllUsernamesResponse{ success: true, message: "success".into(), usernames }
        },
        Err(error) => {
            eprintln!("Error: Getting all usernames failed. Error: {}", error);
            GetAllUsernamesResponse{ success: false, message: "No usernames found: Server error!".into(), usernames : Vec::new() }
        }
    };

    Json(response)
}
