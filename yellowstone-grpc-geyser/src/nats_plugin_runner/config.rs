use {
    once_cell::sync::Lazy,
    serde::Deserialize,
    std::{fs, path::Path},
};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    load_config("nats_config.toml").expect("Failed to load or fallback to default config")
});

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub nats: ConfigNatsServer,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServer {
    #[serde(default = "default_nats_url")]
    pub url: String,
    #[serde(default)]
    pub consumers: ConfigNatsServerConsumers,
    #[serde(default)]
    pub fetchers: ConfigNatsServerFetchers,
    #[serde(default)]
    pub streams: ConfigNatsServerStreams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServerStreams {
    #[serde(default)]
    pub name: NatsServerStreamNames,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsServerStreamNames {
    #[serde(default = "default_account")]
    pub account: String,
    #[serde(default = "default_slot")]
    pub slot: String,
    #[serde(default = "default_transaction")]
    pub transaction: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default = "default_block_metadata")]
    pub block_metadata: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServerConsumers {
    #[serde(default = "default_max_batch_size")]
    pub max_batch_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigNatsServerFetchers {
    #[serde(default = "default_channel_bound")]
    pub channel_bound: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            nats: ConfigNatsServer::default(),
        }
    }
}

impl Default for ConfigNatsServer {
    fn default() -> Self {
        Self {
            url: default_nats_url(),
            consumers: ConfigNatsServerConsumers::default(),
            fetchers: ConfigNatsServerFetchers::default(),
            streams: ConfigNatsServerStreams::default(),
        }
    }
}

impl Default for ConfigNatsServerConsumers {
    fn default() -> Self {
        Self {
            max_batch_size: default_max_batch_size(),
        }
    }
}

impl Default for ConfigNatsServerFetchers {
    fn default() -> Self {
        Self {
            channel_bound: default_channel_bound(),
        }
    }
}

impl Default for ConfigNatsServerStreams {
    fn default() -> Self {
        Self {
            name: NatsServerStreamNames::default(),
        }
    }
}

impl Default for NatsServerStreamNames {
    fn default() -> Self {
        Self {
            account: default_account(),
            slot: default_slot(),
            transaction: default_transaction(),
            entry: default_entry(),
            block_metadata: default_block_metadata(),
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    if path.as_ref().exists() {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    } else {
        println!(
            "Config file {} not found. Using default config.",
            path.as_ref().display()
        );
        Ok(Config {
            ..Default::default()
        })
    }
}

// === Default value providers ===

fn default_nats_url() -> String {
    "nats://localhost:4222".into()
}

fn default_max_batch_size() -> i64 {
    64
}

fn default_channel_bound() -> usize {
    5000
}

fn default_account() -> String {
    "accounts".into()
}

fn default_slot() -> String {
    "slots".into()
}

fn default_transaction() -> String {
    "transactions".into()
}

fn default_entry() -> String {
    "entries".into()
}

fn default_block_metadata() -> String {
    "block_metadatas".into()
}
