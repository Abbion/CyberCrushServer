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
    let password_query = sqlx::query_scalar::<_, String>(
        r#"SELECT password FROM users WHERE username = $1"#
        )
        .bind(&payload.username)
        .fetch_optional(&state.db_pool)
        .await;

    let response = match password_query {
        Ok(Some(stored_password_hash)) => {
            let verified = verify_password(&stored_password_hash, &payload.password, &state.pepper);

            if verified.is_err() {
                eprintln!("Error: Login failed for user: {}. Error: {}", payload.username, verified.unwrap_err());
                LoginResponse{ success: false, message: "Password decodeing error.".into(), token: "".into() }
            }
            else if verified.unwrap() == true {
                LoginResponse{ success: true, message: "success".into(), token: "test Token".into() }
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
            LoginResponse{ success: false, message: "Login database error!".into(), token: "".into() }
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
