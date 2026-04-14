use shared_server_lib::server_configurator::{ServerConfiguration, ServerType};

use axum::{
    extract::{ Json, State },
    response::IntoResponse,
    routing::get,
    Router,
};

use serde::Serialize;
use tokio::net::TcpListener;
use tokio;
use tokio::sync::Mutex;
use crate::tokio::time::{ sleep, Duration };

use std::sync::Arc;

mod app;
use crate::{ app::{App, GameState} };

#[derive(Debug, Serialize)]
struct GameStateResponse {
    is_game_online: bool,
    info_panel_text: String,
}

#[tokio::main]
async fn main() {
    let game_state = Arc::new(Mutex::new(GameState{ is_online: true,
                                                    offline_message_info: "#TR-GAME_IS_OFFLINE".into()
                                                    }));
    
    let server_ready = Arc::new(Mutex::new(false));

    //====================================================

    let game_state_server = game_state.clone();
    let server_ready_server = server_ready.clone();

    let server_handle = tokio::spawn(async move {
        let server_configuration = ServerConfiguration::load("../server.conf");
    
        let socket_addr = server_configuration.get_socket_addr(ServerType::GameState);
        let listener = TcpListener::bind(socket_addr).await.unwrap();
        println!("Game state server running at: {}", socket_addr);

        let app = Router::new()
            .route("/hello", get(hello))
            .route("/game_state", get(get_game_state))
            .with_state(game_state_server);

        {
            let mut lock = server_ready_server.lock().await;
            *lock = true;
        }

        axum::serve(listener, app).await.unwrap();
    });

    println!("Waiting for server to start...");
    loop {
        sleep(Duration::from_secs(1)).await;
        let lock = server_ready.lock().await;

        if *lock == true {
            break;
        }
    }
    println!("The server is still running!");

    let game_state_tui = game_state.clone();
    let tui_handle = tokio::spawn(async move {
        let mut terminal = ratatui::init();
        
        let mut app = App::new(game_state_tui);
        let app_result = app.run(&mut terminal).await;
        
        if let Err(error) = app_result {
            println!("App run returned an error: {}", error);
        }

        ratatui::restore();
    });
    
    server_handle.await.unwrap();
    tui_handle.await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush game state server!"
}

async fn get_game_state(State(state): State<Arc<Mutex<GameState>>>) -> impl IntoResponse {
    let lock = state.lock().await;
    let game_state = GameStateResponse{ is_game_online: lock.is_online, info_panel_text: lock.offline_message_info.clone() };
    Json(game_state)
}
