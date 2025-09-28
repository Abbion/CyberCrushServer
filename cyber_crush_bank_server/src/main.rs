use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database};

use axum::{
    routing::{get, post},
    Router,
};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_posgres_connection_url()).await;
    
    let socket_addr = server_configuration.get_socket_addr(ServerType::Bank);
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("Bank server running at: {}", socket_addr);

    let app = Router::new()
        .route("/", get(hello));

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush bank server!"
}
