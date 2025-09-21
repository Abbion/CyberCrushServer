use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::{fs, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use sqlx::{postgres::PgPoolOptions, PgPool};
use argon2::{Argon2, PasswordHash, password_hash, PasswordVerifier};
use rand::{TryRngCore, rngs::OsRng};
use hex;

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    success: bool,
    message: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct ServerConfiguration {
    database_name: String,
    database_admin_username : String,
    database_admin_password : String,
    database_url : String,
    database_password_pepper: String,
}

impl ServerConfiguration {
    fn get_posgres_connection_url(&self) -> String {
        format!("postgres://{}:{}@{}/{}", self.database_admin_username, self.database_admin_password, self.database_url, self.database_name)
    }

    fn load() -> ServerConfiguration {
        let configuration_data = fs::read_to_string("../server.conf").expect("Failed to load configuration data");
        let server_config : ServerConfiguration = match serde_json::from_str(&configuration_data) {
            Ok(config) => config,
            Err(error) => {
                panic!("Error: Reading server configuration failed: {}", error);
            }
        };

        return server_config;
    }
}

#[derive(Debug)]
struct ServerState {
    pepper: String,
    db_pool: PgPool, //This is thread safe
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load();
    let db_pool = connect_to_database(server_configuration.get_posgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ pepper: server_configuration.database_password_pepper, db_pool });

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/login", post(login))
        .with_state(server_state.clone());

    let addr = SocketAddr::from(([0,0,0,0], 3000));
    println!("Server running at: {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, cyber crush!"
}

async fn login(State(state): State<Arc<ServerState>>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    #[derive(sqlx::FromRow)]
    struct PasswordQuery {
        id: i32,
        password: String
    }

    let password_query = sqlx::query_as::<_, PasswordQuery>(
        r#"SELECT id, password FROM users WHERE username = $1"#
        )
        .bind(&payload.username)
        .fetch_optional(&state.db_pool)
        .await;

    let response = match password_query {
        Ok(Some(password_query)) => {
            let verified = verify_password(&password_query.password, &payload.password, &state.pepper);

            if verified.is_err() {
                eprintln!("Error: Login failed for user: {}. Error: {}", payload.username, verified.unwrap_err());
                LoginResponse{ success: false, message: "Password decodeing error.".into(), token: "".into() }
            }
            else if verified.unwrap() == true {
                match generate_and_store_token(&state.db_pool, password_query.id).await {
                    Ok(token) => {
                        LoginResponse{ success: true, message: "success".into(), token: token }
                    },
                    Err(error) => {
                        LoginResponse{ success: false, message: error, token: "".into() }
                    }
                }
            }
            else {
                LoginResponse{ success: false, message: "Wrong credentials.".into(), token: "".into() }
            }
        },
        Ok(None) => { 
            LoginResponse{ success: false, message: "User not found.".into(), token: "".into() } 
        },
        Err(error) => {
            eprintln!("Error: Login failed for user: {}. Error: {}", payload.username, error);
            LoginResponse{ success: false, message: "Login database error.".into(), token: "".into() }
        }
    };

    Json(response)
}

async fn connect_to_database(db_url: String) -> PgPool {
    let db_pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&db_url)
        .await;

    let db_pool = match db_pool {
        Ok(pool) => pool,
        Err(error) => {
            panic!("Error: Server did not connect to the database: {}", error);
        }
    };

    println!("Connected to postgres!");
    return db_pool;
}

fn verify_password(stored_hash: &str, password: &str, pepper: &str) -> Result<bool, password_hash::Error> {
    let argon2 = Argon2::default();

    let pepper_pass = format!("{}{}", password, pepper);
    let parsed_hash = PasswordHash::new(stored_hash)?;

    Ok(argon2.verify_password(pepper_pass.as_bytes(), &parsed_hash).is_ok())
}

fn generate_token() -> String {
    let mut buffer = [0u8; 32];
    OsRng.try_fill_bytes(&mut buffer).unwrap();
    hex::encode(buffer)
}

async fn generate_and_store_token(db_pool: &PgPool, user_id: i32) -> Result<String, String> {
    const MAX_ATTEMPTS: usize = 8;

    for attempt in 0..MAX_ATTEMPTS {
        let token = generate_token();

        let token_collision = sqlx::query_scalar::<_, i64>("SELECT 1 FROM users WHERE user_token = $1")
            .bind(&token)
            .fetch_optional(db_pool)
            .await;

        let token_collision = match token_collision {
            Ok(state) => state,
            Err(error) => {
                eprintln!("Error: Failed to check token collision for user_id {}: {}", user_id, error);
                return Err("Database error. Token collision.".into());
            }
        };

        if token_collision.is_some() {
            eprintln!("Info: token collision has occured! Retrying attempt {}/{}", attempt + 1, MAX_ATTEMPTS);
            continue;
        }

        let token_update = sqlx::query("UPDATE users SET user_token = $1 WHERE id = $2")
            .bind(&token)
            .bind(&user_id)
            .execute(db_pool)
            .await;

        let update_result = match token_update {
            Ok(result) => result,
            Err(error) => {
                eprintln!("Error: Failed to update the user_id {} token: {}", user_id, error);
                return Err("Database error. Token update error.".into())
            }
        };

        if update_result.rows_affected() == 1 {
            return Ok(token);
        }
        
        eprintln!("Info: Generate token request was completed for user_id {}, but no user was found.", user_id);
        return Err("Database error. No user for token.".into());
    }

    Err("No token yielded.".into())
}
