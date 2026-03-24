use shared_server_lib::server_configurator::{ServerConfiguration, ServerType};

use axum::{
    extract::Json,
    response::IntoResponse,
    routing::get,
    Router,
};

use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Debug, Serialize)]
struct GameStateResponse {
    is_game_online: bool,
    info_panel_text: String,
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    
    let socket_addr = server_configuration.get_socket_addr(ServerType::GameState);
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Game state server running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(hello))
        .route("/game_state", get(game_state));

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush game state server!"
}

async fn game_state() -> impl IntoResponse {
    let game_state = GameStateResponse{ is_game_online: true, info_panel_text: "GAME_IS_OFFLINE".into() };
    Json(game_state)
}
