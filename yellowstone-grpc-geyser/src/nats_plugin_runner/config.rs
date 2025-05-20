use {serde::Deserialize, std::fs};

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

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}
