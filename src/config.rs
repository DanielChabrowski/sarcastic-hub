use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Filesystem {
    pub name: String,
    pub paths: Vec<String>,
    pub extensions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub enum Provider {
    Filesystem(Filesystem),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub web_ui_address: std::net::SocketAddr,

    pub providers: Vec<Provider>,
}

pub fn load_config<P: AsRef<std::path::Path>>(path: P) -> Result<Config> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).map_err(|e| anyhow!("{:?}", e))
}
