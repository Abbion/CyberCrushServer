use std::fs;
use std::net::SocketAddr;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfiguration {
    pub database_name: String,
    database_admin_username: String,
    database_admin_password: String,
    database_url: String,
    pub database_password_pepper: String,
    server_address: String,
    authentication_server_port: u16,
    data_server_port: u16,
    bank_server_port: u16,
    chat_server_port: u16
}

pub enum ServerType {
    Authentication,
    Data,
    Bank,
    Chat
}

impl ServerConfiguration {
    pub fn get_posgres_connection_url(&self) -> String {
        format!("postgres://{}:{}@{}/{}", self.database_admin_username, self.database_admin_password, self.database_url, self.database_name)
    }

    pub fn load(server_configuration_file_path: &str) -> ServerConfiguration {
        let configuration_data = fs::read_to_string(server_configuration_file_path).expect("Failed to load configuration data");
        let server_config: ServerConfiguration = match serde_json::from_str(&configuration_data) {
            Ok(config) => config,
            Err(error) => {
                panic!("Error: Reading server configuration failed: {}", error);
            }
        };

        return server_config;
    }

    pub fn get_socket_addr(&self, server_type: ServerType) -> SocketAddr {
        let addr_str = match server_type {
            ServerType::Authentication => format!("{}:{}", self.server_address, self.authentication_server_port),
            ServerType::Data => format!("{}:{}", self.server_address, self.data_server_port),
            ServerType::Bank => format!("{}:{}", self.server_address, self.bank_server_port),
            ServerType::Chat => format!("{}:{}", self.server_address, self.chat_server_port),
        };

        addr_str.parse().expect("Invalid ip address")
    }
}
