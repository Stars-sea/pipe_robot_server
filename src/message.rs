use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::watch::{self, Receiver, Sender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    from: String,
    to: String,
    body: String,
}

impl Message {
    pub fn new(from: String, to: String, body: String) -> Message {
        Message { from, to, body }
    }

    pub fn body(&self) -> &String {
        &self.body
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| String::from("{}"))
    }

    fn empty() -> Message {
        Message::new("none".to_string(), "none".to_string(), "none".to_string())
    }
}

impl ToString for Message {
    fn to_string(&self) -> String {
        format!("[{} -> {}]: {}", self.from, self.to, self.body())
    }
}

#[derive(Debug, Clone)]
pub struct MessagePipe {
    tx: Arc<Sender<Message>>,
    rx: Receiver<Message>,
}

impl MessagePipe {
    pub fn new() -> MessagePipe {
        let (tx, rx) = watch::channel(Message::empty());
        MessagePipe {
            tx: Arc::new(tx),
            rx: rx,
        }
    }

    pub fn send(&self, item: Message) -> Result<()> {
        Ok(self.tx.send(item)?)
    }

    pub async fn get(&mut self, name: &String) -> Result<Message> {
        self.rx.mark_unchanged();
        loop {
            self.rx.changed().await?;
            let message = (*self.rx.borrow()).clone();
            if message.to != "all" && message.to != *name {
                continue;
            }
            return Ok(message);
        }
    }
}
