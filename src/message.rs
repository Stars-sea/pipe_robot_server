use std::sync::Arc;

use anyhow::Result;
use tokio::sync::watch::{self, Receiver, Sender};

#[derive(Debug, Clone)]
pub struct Message {
    from: String,
    to: String,
    body: String,
}

impl Message {
    pub fn new(from: &String, to: &String, body: &String) -> Message {
        Message {
            from: from.clone(),
            to: to.clone(),
            body: body.clone(),
        }
    }

    pub fn body(&self) -> &String {
        &self.body
    }

    fn empty() -> Message {
        Message::new(
            &"none".to_string(),
            &"none".to_string(),
            &"none".to_string(),
        )
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
        loop {
            self.rx.changed().await?;
            let message = (*self.rx.borrow()).clone();
            if message.to != *name {
                continue;
            }
            return Ok(message);
        }
    }
}
