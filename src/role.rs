use anyhow::{Result, anyhow};
use tokio::sync::RwLock;

use crate::packet::Packet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Controller(String),
    Receiver(String),
    Unknown,
}

impl Role {
    pub fn name(&self) -> Result<String> {
        match &self {
            Role::Controller(name) => Ok(name.clone()),
            Role::Receiver(name) => Ok(name.clone()),
            Role::Unknown => Err(anyhow!("Unknown role")),
        }
    }

    pub fn name_or_unknown(&self) -> String {
        self.name().unwrap_or("unknown_role".to_string())
    }
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match &self {
            Role::Controller(name) => format!("controller:{}", name),
            Role::Receiver(name) => format!("receiver:{}", name),
            Role::Unknown => "unknown".to_string()
        }
    }
}

pub trait RoleExt {
    fn new_packet(&self, body: String) -> Packet;

    fn new_packet_with_id(&self, body: String, id: String) -> Packet;
}

impl RoleExt for Role {
    fn new_packet(&self, body: String) -> Packet {
        Packet::new(vec![self.name_or_unknown()], body)
    }

    fn new_packet_with_id(&self, body: String, id: String) -> Packet {
        Packet::new_with_id(vec![self.name_or_unknown()], body, id)
    }
}

pub struct RoleContainer {
    roles: RwLock<Vec<Role>>,
}

impl RoleContainer {
    pub fn new() -> RoleContainer {
        RoleContainer {
            roles: RwLock::new(Vec::new()),
        }
    }

    pub async fn add(&self, role: Role) -> bool {
        if role == Role::Unknown || self.roles.read().await.contains(&role) {
            return false;
        }

        let mut lock = self.roles.write().await;
        lock.push(role);
        return true;
    }

    pub async fn remove(&self, role: &Role) -> bool {
        let mut lock = self.roles.write().await;
        if let Some(pos) = lock.iter().position(|r| r == role) {
            lock.remove(pos);
            return true;
        }

        return false;
    }

    pub async fn list(&self) -> Vec<Role> {
        self.roles.read().await.clone()
    }

    pub async fn list_controllers(&self) -> Vec<String> {
        self.roles
            .read()
            .await
            .iter()
            .filter_map(|r| {
                if let Role::Controller(name) = r {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub async fn list_receivers(&self) -> Vec<String> {
        self.roles
            .read()
            .await
            .iter()
            .filter_map(|r| {
                if let Role::Receiver(name) = r {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
