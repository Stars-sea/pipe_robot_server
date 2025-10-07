use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use log::{debug, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{interval, timeout};

use crate::handler::Context;

async fn hearbeat_handler(stream: Arc<Mutex<&mut TcpStream>>, role_info: String) -> Result<()> {
    let mut heartbeat_interval = interval(Duration::from_secs(5));

    loop {
        heartbeat_interval.tick().await;

        let mut buf = [0; 128];

        {
            if let Err(e) = stream.lock().await.write_all(b"HEARTBEAT").await {
                warn!("Failed to send heartbeat to {}: {:?}", role_info, e);
                return Err(e.into());
            }
            debug!("Sent heartbeat to {}", role_info);
        }

        match timeout(Duration::from_secs(1), stream.lock().await.read(&mut buf)).await? {
            Ok(0) => {
                info!("Connection closed by {}", role_info);
                return Ok(());
            }
            Ok(n) => {
                let received_data = String::from_utf8_lossy(&buf[..n]).to_string();
                if received_data == "HEARTBEAT_ACK" {
                    debug!("Received hearbeat from {}", role_info);
                } else {
                    return Err(anyhow!("Received invalid heartbeat ack"));
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

async fn notify_handler(stream: Arc<Mutex<&mut TcpStream>>, ctx: Context) -> Result<()> {
    let Context {
        role,
        addr,
        others: _,
        mut pipe,
    } = ctx;

    let role_name = role.name()?;
    loop {
        let received_msg = pipe.get(&role_name).await;
        if let Err(e) = &received_msg {
            warn!("Received bad message: {:?}", e);
            continue;
        }

        let received_msg = received_msg.unwrap();
        debug!("Received message from pipe: {}", received_msg.to_string());

        stream
            .lock()
            .await
            .write(received_msg.body().as_bytes())
            .await?;
        info!(
            "Send message to {}({}): {:?}",
            role_name,
            addr,
            received_msg.body()
        );
    }
}

pub(super) async fn receiver_handler(stream: &mut TcpStream, ctx: Context) -> Result<()> {
    let stream = Arc::new(Mutex::new(stream));
    let stream_cloned = Arc::clone(&stream);

    tokio::select! {
        heartbeat_result = hearbeat_handler(stream, ctx.role_info()) => {
            return heartbeat_result;
        }

        notify_result = notify_handler(stream_cloned, ctx.clone()) => {
            return notify_result;
        }
    }
}
