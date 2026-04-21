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

#[derive(Debug, Deserialize)]
struct GetHackerInfoRequest {
    personal_code: String
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct HackerInfo {
    username: String,
    token: String,
    can_hack: bool,
}

#[derive(Debug, Serialize)]
struct GetHackerInfoResponse {
    response_status: ResponseStatus,
    hacker_info: HackerInfo,
}

impl GetHackerInfoResponse {
    fn fail(reason: &str) -> GetHackerInfoResponse {
        GetHackerInfoResponse{
            response_status: ResponseStatus::fail(reason.into()),
            hacker_info: HackerInfo{ username: "none".into(), token: "none".into(), can_hack: false } }
    }

    fn success(hacker_info: HackerInfo) -> GetHackerInfoResponse {
        GetHackerInfoResponse{
            response_status: ResponseStatus::success(),
            hacker_info }
    }
}

#[derive(Debug)]
struct ServerState {
    db_pool: PgPool, //This is thread safe
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_postgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ db_pool });

    let socket_addr = server_configuration.get_socket_addr(ServerType::Hacking);

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Bank hacking running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(hello))
        .route("/get_hacker_info", post(get_hacker_info))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush hacking server!"
}

async fn get_hacker_info(State(state): State<Arc<ServerState>>, Json(payload): Json<GetHackerInfoRequest>) -> impl IntoResponse {
    let user_personal_code = match payload.personal_code.parse::<i32>() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: Hacker code failed to parse: {}, error: {}", payload.personal_code, error);
            return Json(GetHackerInfoResponse::fail("Personal code parsing failed"));
        }
    };

    let hacker_query = sqlx::query_as::<_, HackerInfo> (
    r#"
        SELECT username, token, can_hack FROM users
        WHERE personal_code = $1
    "#)
    .bind(&user_personal_code)
    .fetch_optional(&state.db_pool)
    .await;

    let response = match hacker_query {
        Ok(Some(hacker_info)) => {
            if hacker_info.can_hack == true {
                GetHackerInfoResponse::success(hacker_info)
            }
            else {
                GetHackerInfoResponse::fail("No hacking privilege")
            }
        },
        Ok(None) => GetHackerInfoResponse::fail("User not found"),
        Err(error) => {
            eprintln!("Error: Getting hacker failed for personal code: {}, error: {}", payload.personal_code, error);
            GetHackerInfoResponse::fail("User not found. Server error!")
        }
    };

    Json(response) 
}


// Get hacker data
// Get list of victims with cyber defence level
// Get hacking options for selected victim
// Get victim token
