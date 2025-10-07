mod controller_handler;
mod receiver_handler;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use log::{error, info, warn};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::message::MessagePipe;
use crate::packet::TcpStreamExt;
use crate::role::{Role, RoleContainer};
use controller_handler::controller_handler;
use receiver_handler::receiver_handler;

#[derive(Clone)]
struct Context {
    role: Role,
    addr: SocketAddr,
    others: Arc<RoleContainer>,
    pipe: MessagePipe,
}

impl Context {
    fn new(
        role: &Role,
        addr: SocketAddr,
        roles: &Arc<RoleContainer>,
        pipe: MessagePipe,
    ) -> Context {
        Context {
            role: role.clone(),
            addr: addr,
            others: roles.clone(),
            pipe: pipe,
        }
    }
}

async fn verify_connection(stream: &mut TcpStream) -> Result<Role> {
    let msg = stream.read_string().await?;

    if let Some(name) = msg.strip_prefix("controller:") {
        return Ok(Role::Controller(name.to_string()));
    }
    if let Some(name) = msg.strip_prefix("receiver:") {
        return Ok(Role::Receiver(name.to_string()));
    }

    Ok(Role::Unknown)
}

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    roles: Arc<RoleContainer>,
    pipe: MessagePipe,
) {
    let verify_result = verify_connection(&mut stream).await;
    if let Err(e) = verify_result {
        error!("Failed to verify connection {}: {:?}", addr, e);
        stream.shutdown().await.ok();
        return;
    }

    let role = verify_result.unwrap();
    let ctx = Context::new(&role, addr, &roles, pipe);

    roles.add(role.clone()).await;

    let result = match &role {
        Role::Controller(name) => {
            info!("Connection {} identified as Controller: {}", addr, name);
            controller_handler(&mut stream, ctx).await
        }
        Role::Receiver(name) => {
            info!("Connection {} identified as Receiver: {}", addr, name);
            receiver_handler(&mut stream, ctx).await
        }
        Role::Unknown => Err(anyhow!("Connection {} has unknown role", addr)),
    };

    if let Err(e) = result {
        warn!("Connection closed: {:?}", e);
    }

    stream.shutdown().await.ok();
    roles.remove(&role).await;
}
