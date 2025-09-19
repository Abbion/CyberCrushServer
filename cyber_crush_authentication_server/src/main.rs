use axum::{
    extract::Json,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    username: String,
    password: String,
}

#[tokio::main]
async fn main() {
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect("postgres://admin_user:secret@localhost/cc_game")
        .await;

    println!("Connected to postgres");

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/login", post(login));

    let addr = SocketAddr::from(([0,0,0,0], 3000));
    println!("Server running at: {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, cyber crush!"
}

async fn login(Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    let response = LoginResponse {
        username: format!("{}@", payload.username),
        password: format!("{}@", payload.password),
    };

    Json(response)
}
