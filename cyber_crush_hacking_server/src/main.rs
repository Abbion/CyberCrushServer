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
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
struct GetHackerInfoRequest {
    personal_number: String
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct HackerInfo {
    username: String,
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
            hacker_info: HackerInfo{ username: "".into(), can_hack: false } }
    }

    fn success(hacker_info: HackerInfo) -> GetHackerInfoResponse {
        GetHackerInfoResponse{
            response_status: ResponseStatus::success(),
            hacker_info }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct HackableUser {
    username: String,
    cyber_defence_level: i32,
    personal_number: i32,
}

#[derive(Debug, Serialize)]
struct GetHackableUsersResponse {
    response_status: ResponseStatus,
    users: Vec::<HackableUser>,
}

#[derive(Debug, Serialize, Deserialize)]
enum HackType {
    BankTransaction(i32),
    ChatAccess,
    ChatData(i32),
    PersonalData
}

#[derive(Debug, Deserialize)]
struct AvailableHackTypesRequest {
    hacker_personal_number: String,
    victim_personal_number: String,
}

#[derive(Debug, Serialize)]
struct AvailableHackTypesResponse {
    response_status: ResponseStatus,
    available_hacks: Vec::<HackType>,
}

#[derive(Debug, Deserialize)]
struct HackTokenRequest {
    victim_personal_number: String,
}

#[derive(Debug, Serialize)]
struct HackTokenResponse {
    response_status: ResponseStatus,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HackStateResultRequest {
    hacker_personal_number: String,
    victim_personal_number: String,
    hack_type: HackType,
    hack_successful: bool
}

use ResponseStatus as HackStateResultResponse;

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
        .route("/get_hackable_users", get(get_hackable_users))
        .route("/get_available_hack_types", post(get_available_hack_types))
        .route("/get_hack_token", post(get_hack_token))
        .route("/log_hack_state_result", post(log_hack_state_result))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush hacking server!"
}

async fn get_hacker_info(State(state): State<Arc<ServerState>>, Json(payload): Json<GetHackerInfoRequest>) -> impl IntoResponse {
    let user_personal_number = parse_personal_number(&payload.personal_number);
    if user_personal_number < 1 {
        return Json(GetHackerInfoResponse::fail("Hacker personal code parsing failed"));
    }

    let hacker_query = sqlx::query_as::<_, HackerInfo> (
    r#"
        SELECT username, can_hack FROM users
        WHERE personal_number = $1
    "#)
    .bind(&user_personal_number)
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
            eprintln!("Error: Getting hacker failed for personal code: {}, error: {}", payload.personal_number, error);
            GetHackerInfoResponse::fail("User not found. Internal server error!")
        }
    };

    Json(response) 
}

async fn get_hackable_users(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let hackable_users_query = sqlx::query_as::<_, HackableUser> (
    r#"
        SELECT username, cyber_defence_level, personal_number FROM users
        WHERE can_hack = false
    "#)
    .fetch_all(&state.db_pool)
    .await;
    
    let response = match hackable_users_query {
        Ok(users) => GetHackableUsersResponse{ response_status: ResponseStatus::success(), users },
        Err(error) => {
            eprintln!("Error: Getting hackable users failed, error: {}", error);
            GetHackableUsersResponse{ response_status: ResponseStatus::fail("Hackable users not found. Inertnal server error!".into()), users: vec![] }
        }
    };

    Json(response)
}

async fn get_available_hack_types(State(state): State<Arc<ServerState>>, Json(payload): Json<AvailableHackTypesRequest>) -> impl IntoResponse {
    #[derive(Debug, sqlx::FromRow)]
    struct HackQueryResult {
        hack_type: sqlx::types::Json<HackType>,
        successful: bool,
    }

    let hacker_personal_number = parse_personal_number(&payload.hacker_personal_number);
    if hacker_personal_number < 1 {
        return Json(AvailableHackTypesResponse{ response_status: ResponseStatus::fail("Hacker personal code is not valid. Cannot gather available hacks.".into()), available_hacks: vec![] });
    }

    let victim_personal_number = parse_personal_number(&payload.victim_personal_number);
    if victim_personal_number < 1 {
        return Json(AvailableHackTypesResponse{ response_status: ResponseStatus::fail("Victim personal code is not valid. Cannot gather available hacks.".into()), available_hacks: vec![] });
    }

    let query_result = sqlx::query_as::<_, HackQueryResult>(
    r#"
        SELECT
            hl.hack_type,
            hl.successful
        FROM hack_log hl
        INNER JOIN users hacker ON hl.hacker_id = hacker.id
        INNER JOIN users victim ON hl.victim_id = victim.id
        WHERE hacker.personal_number = $1
            AND victim.personal_number = $2;
    "#
    )
    .bind(&hacker_personal_number)
    .bind(&victim_personal_number)
    .fetch_all(&state.db_pool)
    .await;

    let hacks = match query_result {
        Ok(hacks) => hacks,
        Err(error) => {
            eprintln!("Failed to query hacker available hacks for: {}, error: {}", hacker_personal_number, error);
            return Json(AvailableHackTypesResponse{ response_status: ResponseStatus::fail("Querying available hacks failed. Internal server error.".into()), available_hacks: vec![] });
        }
    };

    let mut can_hack_bank: bool = true;
    let mut can_hack_chats: bool = false;
    let mut available_hacks = vec![HackType::PersonalData];

    for hack in hacks {
        match hack.hack_type.0 {
            HackType::BankTransaction(_) => {
                if hack.successful == true {
                    can_hack_bank = false;
                }
            },
            HackType::ChatAccess => {
                if hack.successful == true {
                    can_hack_chats = true;
                }
            },
            _ => {}
        }
    }

    if can_hack_bank == true {
        available_hacks.push(HackType::BankTransaction(0));
    }

    if can_hack_chats == true {
        available_hacks.push(HackType::ChatData(0));
    }
    else {
        available_hacks.push(HackType::ChatAccess);
    }

    Json(AvailableHackTypesResponse{ response_status: ResponseStatus::success(), available_hacks })
}

async fn get_hack_token(State(state): State<Arc<ServerState>>, Json(payload): Json<HackTokenRequest>) -> impl IntoResponse {
    let victim_personal_number = parse_personal_number(&payload.victim_personal_number);
    if victim_personal_number < 1 {
        return Json(HackTokenResponse{ response_status: ResponseStatus::fail("Victim personal code parsing failed.".into()), token: None });
    }

    let token_query = sqlx::query_scalar::<_, Option::<String>>(
    r#"
        SELECT user_token FROM users
        WHERE personal_number = $1
    "#
    )
    .bind(&victim_personal_number)
    .fetch_optional(&state.db_pool)
    .await;
    
    let response = match token_query {
        Ok(Some(token)) => {
            HackTokenResponse{ response_status: ResponseStatus::success(), token }
        },
        Ok(None) => {
            HackTokenResponse{ response_status: ResponseStatus::fail("Hack token was not found. Personal code is not assigned to a user.".into()), token: None }
        },
        Err(error) => {
            eprintln!("Error: Getting hack token failed for code: {}, error: {}", victim_personal_number, error);
            HackTokenResponse{ response_status: ResponseStatus::fail("Hack token not found. Internal server error!".into()), token: None }
        }
    };

    Json(response)
}

async fn log_hack_state_result(State(state): State<Arc<ServerState>>, Json(payload): Json<HackStateResultRequest>) -> impl IntoResponse {
    let hacker_personal_number = parse_personal_number(&payload.hacker_personal_number);
    if hacker_personal_number < 1 {
        return Json(HackStateResultResponse::fail("Hacker personal code is not valid. Cannot log hack.".into()));
    }

    let victim_personal_number = parse_personal_number(&payload.victim_personal_number);
    if victim_personal_number < 1 {
        return Json(HackStateResultResponse::fail("Victim personal code is not valid. Cannot log hack.".into()));
    }

    let insertion_result = sqlx::query(
    r#"
        INSERT INTO hack_log(
            hacker_id,
            victim_id,
            hack_type,
            successful,
            timestamp
        )
        SELECT
            hacker_user.id AS hacker_id,
            victim_user.id AS victim_id,
            $1,
            $2,
            NOW()
        FROM users AS hacker_user, users AS victim_user
        WHERE hacker_user.personal_number = $3
            AND victim_user.personal_number = $4
        RETURNING id;
    "#
    )
    .bind(sqlx::types::Json(&payload.hack_type))
    .bind(payload.hack_successful)
    .bind(hacker_personal_number)
    .bind(victim_personal_number)
    .fetch_one(&state.db_pool)
    .await;
    
    let response = match insertion_result {
        Ok(_) => {
            HackStateResultResponse::success()
        },
        Err(error) => {
            eprintln!("Hack log insertion failed. Error: {}", error);
            HackStateResultResponse::fail("Hack log caused a internal server error!".into())
        }
    };
    
    Json(response)
}

fn parse_personal_number(personal_number: &String) -> i32 {
    match personal_number.parse::<i32>() {
        Ok(code) => { 
            return code;
        },
        Err(error) => {
            eprintln!("Error: Hacker code failed to parse: {}, error: {}", personal_number, error);
            return 0;
        }
    };
}
