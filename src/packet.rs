use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub struct Packet {
    pub receivers: Vec<String>,
    pub body: String,
    pub id: String,
}

impl Packet {
    pub fn new(receivers: Vec<String>, body: String) -> Packet {
        Packet { receivers, body, id: Uuid::new_v4().to_string() }
    }

    pub fn new_with_id(receivers: Vec<String>, body: String, id: String) -> Packet {
        Packet { receivers, body, id }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| String::from("{}"))
    }

    pub fn from_json(json_str: &str) -> Result<Packet> {
        Ok(serde_json::from_str(json_str)?)
    }
}

pub trait TcpStreamExt {
    async fn read_string(&mut self) -> Result<String>;

    async fn read_packet(&mut self) -> Result<Packet>;

    async fn write_packet(&mut self, packet: &Packet) -> Result<()>;
}

impl TcpStreamExt for TcpStream {
    async fn read_string(&mut self) -> Result<String> {
        let mut msg = vec![0; 1024];
        let read_size = self.read(&mut msg).await?;
        if read_size == 0 {
            return Err(anyhow!("Connection was closed"));
        }

        msg.truncate(read_size);
        return Ok(String::from_utf8(msg)?);
    }

    async fn read_packet(&mut self) -> Result<Packet> {
        let msg = self.read_string().await?;
        Ok(Packet::from_json(&msg)?)
    }

    async fn write_packet(&mut self, packet: &Packet) -> Result<()> {
        self.write(packet.to_json().as_bytes()).await?;
        self.write(b"\n").await?;
        self.flush().await?;
        Ok(())
    }
}
