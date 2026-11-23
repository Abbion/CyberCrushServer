use shared_server_lib::server_configurator::{ServerConfiguration, ServerType};

use axum::{
    extract::{ Json, Request, State },
    http::{ header, StatusCode },
    middleware::{ self, Next },
    response::{ Html, IntoResponse, Response },
    routing::{ get, post },
    Router,
};

use base64::{ engine::general_purpose::STANDARD, Engine };

use serde::{ Deserialize, Serialize };
use tokio::net::TcpListener;
use tokio;
use tokio::sync::Mutex;
use crate::tokio::time::{ sleep, Duration };

use std::io::IsTerminal;
use std::sync::Arc;

mod app;
use crate::{ app::{App, GameState} };

#[derive(Debug, Serialize)]
struct GameStateResponse {
    is_game_online: bool,
    info_panel_text: String,
}

#[derive(Debug, Deserialize)]
struct GameStateUpdateRequest {
    is_game_online: Option<bool>,
    info_panel_text: Option<String>,
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

        let expected_auth_header: Option<Arc<String>> = if server_configuration.admin_username.is_empty() || server_configuration.admin_password.is_empty() {
            eprintln!("Error: admin_username or admin_password is not set in the server configuration, admin endpoints are disabled");
            None
        }
        else {
            let credentials = format!("{}:{}", server_configuration.admin_username, server_configuration.admin_password);
            Some(Arc::new(format!("Basic {}", STANDARD.encode(credentials))))
        };

        let admin_routes = Router::new()
            .route("/admin", get(admin_page))
            .route("/admin/game_state", post(update_game_state))
            .route_layer(middleware::from_fn_with_state(expected_auth_header, require_basic_auth));

        let app = Router::new()
            .route("/hello", get(hello))
            .route("/game_state", get(get_game_state))
            .merge(admin_routes)
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

    if std::io::stdin().is_terminal() {
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

        if let Err(error) = tui_handle.await {
            eprintln!("Error: Terminal interface task failed: {}", error);
        }
    }
    else {
        println!("No terminal detected, use the web interface at /admin");
    }

    if let Err(error) = server_handle.await {
        eprintln!("Error: Game state server task failed: {}", error);
    }
}

async fn hello() -> &'static str {
    "Hello, cyber crush game state server!"
}

async fn get_game_state(State(state): State<Arc<Mutex<GameState>>>) -> impl IntoResponse {
    let lock = state.lock().await;
    let game_state = GameStateResponse{ is_game_online: lock.is_online, info_panel_text: lock.offline_message_info.clone() };
    Json(game_state)
}

async fn admin_page() -> Html<&'static str> {
    Html(include_str!("admin.html"))
}

async fn update_game_state(State(state): State<Arc<Mutex<GameState>>>, Json(payload): Json<GameStateUpdateRequest>) -> impl IntoResponse {
    let mut lock = state.lock().await;

    if let Some(is_online) = payload.is_game_online {
        lock.is_online = is_online;
    }

    if let Some(text) = payload.info_panel_text {
        if !text.trim().is_empty() {
            lock.offline_message_info = text;
        }
    }

    Json(GameStateResponse{ is_game_online: lock.is_online, info_panel_text: lock.offline_message_info.clone() })
}

fn constant_time_equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter().zip(b.iter()).fold(0u8, |difference, (x, y)| difference | (x ^ y)) == 0
}

async fn require_basic_auth(State(expected): State<Option<Arc<String>>>, request: Request, next: Next) -> Response {
    let Some(expected) = expected else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let is_authorized = request.headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| constant_time_equals(value.as_bytes(), expected.as_bytes()))
        .unwrap_or(false);

    if is_authorized {
        next.run(request).await
    }
    else {
        (StatusCode::UNAUTHORIZED, [(header::WWW_AUTHENTICATE, "Basic realm=\"CyberCrush admin\"")]).into_response()
    }
}
