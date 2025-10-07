use std::sync::Arc;

use anyhow::{Result, anyhow};
use log::{debug, info};
use tokio::net::TcpStream;

use crate::handler::Context;
use crate::message::Message;
use crate::packet::{Packet, TcpStreamExt};
use crate::role::{Role, RoleContainer, RoleExt};

async fn command_handler(command: String, others: Arc<RoleContainer>) -> Result<Option<String>> {
    match command.as_str() {
        "list" => {
            let list: Vec<String> = others.list().await.iter().map(|r| r.to_string()).collect();
            Ok(Some(format!("[{}]", list.join(","))))
        }
        "list_controllers" => {
            let list: Vec<String> = others.list_controllers().await;
            Ok(Some(format!("[{}]", list.join(","))))
        }
        "list_receivers" => {
            let list: Vec<String> = others.list_receivers().await;
            Ok(Some(format!("[{}]", list.join(","))))
        }
        _ => Ok(None),
    }
}

async fn command_packet_handler(
    stream: &mut TcpStream,
    packet: Packet,
    role: &Role,
    role_info: String,
    others: Arc<RoleContainer>,
) -> Result<()> {
    let command = packet.body;
    info!("Received command from {}: {:?}", role_info, command);
    if let Some(resp) = command_handler(command.clone(), Arc::clone(&others)).await? {
        info!("Respond to {}: {:?}", role_info, &resp);

        let resp_message = Message::new("server".to_string(), role.name_or_unknown(), resp);
        let resp_packet = role.new_packet_with_id(resp_message.to_json(), packet.id);
        stream.write_packet(&resp_packet).await?;
    } else {
        info!("Unknown command, ignored");
    }
    Ok(())
}

pub(super) async fn controller_handler(stream: &mut TcpStream, ctx: Context) -> Result<()> {
    let Context {
        role,
        addr: _,
        others,
        pipe,
    } = &ctx;

    let role_name = role.name()?;
    let role_info = ctx.role_info();
    loop {
        let packet = stream.read_packet().await;

        if let Err(e) = &packet {
            debug!("Received packet from {}, ignored: {:?}", role_info, e);

            let packet = role.new_packet(format!("Error reading packet: {:?}", e));
            stream.write_packet(&packet).await.ok();
            return Err(anyhow!("Error reading packet: {:?}", e));
        }

        let packet = packet.unwrap();
        info!(
            "Received packet from {}: {:?}",
            role_info,
            packet.to_string()
        );

        if packet.receivers.len() == 1 && packet.receivers.contains(&"server".to_string()) {
            command_packet_handler(stream, packet, role, ctx.role_info(), Arc::clone(others))
                .await?;
            continue;
        }

        packet.receivers.iter().for_each(|receiver| {
            let msg = Message::new(role_name.clone(), receiver.to_string(), packet.body.clone());
            pipe.send(msg).ok();
        });
    }
}
