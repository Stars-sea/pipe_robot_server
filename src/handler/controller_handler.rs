use std::sync::Arc;

use anyhow::{Result, anyhow};
use log::{Level, debug, info, log_enabled};
use tokio::net::TcpStream;

use crate::handler::Context;
use crate::message::Message;
use crate::packet::TcpStreamExt;
use crate::role::{RoleContainer, RoleExt};

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

pub(super) async fn controller_handler(stream: &mut TcpStream, ctx: Context) -> Result<()> {
    let Context {
        role,
        addr,
        others,
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
            return Err(anyhow!("Error reading packet: {:?}", e));
        }

        let packet = packet.unwrap();
        if packet.receivers.len() == 1 && packet.receivers.contains(&"server".to_string()) {
            let command = packet.body;
            info!(
                "Received command from {}({}): {:?}",
                role_name, addr, command
            );
            if let Some(resp) = command_handler(command.clone(), others.clone()).await? {
                info!("Respond to {}({}): {:?}", role_name, addr, &resp);

                let resp_message = Message::new(&"server".to_string(), &role_name, &resp);
                let resp_packet = role.new_packet_with_id(resp_message.to_json(), packet.id);
                stream.write_packet(&resp_packet).await?;
            } else {
                info!("Unknown command, ignored");
            }
            continue;
        }

        packet.receivers.iter().for_each(|receiver| {
            let msg = Message::new(&role_name, &receiver, &packet.body);
            pipe.send(msg).ok();
        });
    }
}
