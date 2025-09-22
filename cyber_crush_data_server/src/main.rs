use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::PgPool;

#[derive(Debug)]
struct ServerState {
    pepper: String,
    db_pool: PgPool, //This is thread safe
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_posgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ pepper: server_configuration.database_password_pepper.clone(), db_pool });
    
    let socket_addr = server_configuration.get_socket_addr(ServerType::Data);
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Data server running at: {}", socket_addr);

    let app = Router::new()
        .route("/", get(hello))
        .with_state(server_state.clone());

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush data server!"
}
