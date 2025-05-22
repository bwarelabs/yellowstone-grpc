use {
    serde::Deserialize,
    std::{fs, path::Path},
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub nats: ConfigNatsServer,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServer {
    pub url: String,
    pub streams: ConfigNatsServerStreams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServerStreams {
    pub account_stream_name: String,
    pub slot_stream_name: String,
    pub transaction_stream_name: String,
    pub entry_stream_name: String,
    pub block_metadata_stream_name: String,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}
