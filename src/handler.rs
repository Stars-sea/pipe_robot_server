use anyhow::{Result, anyhow};
use log::{Level, debug, error, info, log_enabled, warn};
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::message::{Message, MessagePipe};
use crate::packet::TcpStreamExt;
use crate::role::{Role, RoleContainer, RoleExt};

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

async fn verify_connection(stream: &TcpStream) -> Result<Role> {
    let msg = stream.read_string().await?;

    if let Some(name) = msg.strip_prefix("controller:") {
        return Ok(Role::Controller(name.to_string()));
    }
    if let Some(name) = msg.strip_prefix("receiver:") {
        return Ok(Role::Receiver(name.to_string()));
    }

    Ok(Role::Unknown)
}

async fn handle_controller(stream: &mut TcpStream, ctx: Context) -> Result<()> {
    let Context {
        role,
        addr,
        others: _,
        pipe,
    } = ctx;

    let role_name = role.name()?;
    loop {
        let packet = stream.read_packet().await;
        if log_enabled!(Level::Debug)
            && let Ok(packet) = &packet
        {
            debug!(
                "Received packet from {}({}): {:?}",
                role_name,
                addr,
                packet.to_json()
            );
        }

        if let Err(e) = &packet {
            debug!(
                "Received packet from {}({}), ignored: {:?}",
                role_name, addr, e
            );

            let packet = role.new_packet(format!("Error reading packet: {:?}", e));
            stream.write_packet(&packet).await.ok();
            continue;
        }

        let packet = packet.unwrap();
        packet.receivers.iter().for_each(|receiver| {
            let msg = Message::new(&role_name, &receiver, &packet.body);
            pipe.send(msg).ok();
        });
    }
}

async fn handle_receiver(stream: &mut TcpStream, ctx: Context) -> Result<()> {
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
        debug!("Received message: {}", received_msg.to_string());

        stream.write(received_msg.body().as_bytes()).await?;
        debug!("Send message to {}({})", role_name, addr);
    }
}

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    roles: Arc<RoleContainer>,
    pipe: MessagePipe,
) {
    let verify_result = verify_connection(&stream).await;
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
            handle_controller(&mut stream, ctx).await
        }
        Role::Receiver(name) => {
            info!("Connection {} identified as Receiver: {}", addr, name);
            handle_receiver(&mut stream, ctx).await
        }
        Role::Unknown => Err(anyhow!("Connection {} has unknown role", addr)),
    };

    if let Err(e) = result {
        warn!("Connection closed: {:?}", e);

        let err_packet = role.new_packet(format!("Connection closed, error: {:?}", e));
        stream.write_packet(&err_packet).await.ok();
    }

    stream.shutdown().await.ok();
    roles.remove(&role).await;
}
