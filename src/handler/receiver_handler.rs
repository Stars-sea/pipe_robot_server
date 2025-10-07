use anyhow::Result;
use log::{debug, info, warn};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::handler::Context;

pub(super) async fn receiver_handler(stream: &mut TcpStream, ctx: Context) -> Result<()> {
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

        stream.write(received_msg.body().as_bytes()).await?;
        info!(
            "Send message to {}({}): {:?}",
            role_name,
            addr,
            received_msg.body()
        );
    }
}
