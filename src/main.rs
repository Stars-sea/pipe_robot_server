mod config;
mod handler;
mod message;
mod packet;
mod role;

use log::{error, info};
use std::sync::Arc;
use tokio::net::TcpListener;

use config::TcpServerConfig;
use handler::handle_connection;
use message::MessagePipe;
use role::RoleContainer;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = TcpServerConfig::get_config("config.json").unwrap();

    info!("Starting server...");
    let listener = TcpListener::bind(&config.addr).await.unwrap();
    if config.ttl != 0 {
        listener.set_ttl(config.ttl).unwrap();
    }

    info!("Server listening on {}, ttl: {}", config.addr, config.ttl);

    let roles = Arc::new(RoleContainer::new());
    let pipe = MessagePipe::new();

    loop {
        let accpet_result = listener.accept().await;
        if let Err(e) = accpet_result {
            error!("Failed to accept connection: {:?}", e);
            continue;
        }

        let (stream, addr) = accpet_result.unwrap();
        info!("New connection from {}", addr);

        let roles_clone = Arc::clone(&roles);
        let pipe_clone = pipe.clone();
        tokio::spawn(async move {
            handle_connection(stream, addr, roles_clone, pipe_clone).await;
        });
    }
}
