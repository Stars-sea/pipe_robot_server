use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader};

#[derive(Serialize, Deserialize)]
pub struct TcpServerConfig {
    pub addr: String,
    pub whitelist: Vec<String>,
}

impl TcpServerConfig {
    pub fn get_config(config_file: &str) -> Result<TcpServerConfig> {
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);

        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }
}
