use {
    yellowstone_grpc_geyser::{
        nats_geyser_plugin_interface::NatsGeyserPlugin,
        nats_plugin_runner::{
            config::load_config,
            worker::start_stream_workers
        },
        plugin::Plugin
    },
    async_nats::{connect, jetstream},
    std::sync::Arc,
    tokio::signal,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load TOML config for NATS
    let config = load_config("nats_config.toml")?;

     // Load JSON config for plugin
     let mut plugin = Plugin::default();
     plugin.on_load("config.json", false)?;
     let plugin_arc = Arc::new(plugin);

    let client = connect(&config.nats.url).await?;
    let js = jetstream::new(client);

    start_stream_workers("account", &config.nats.streams.account_stream_name, js.clone(), plugin_arc.clone()).await?;
    start_stream_workers("slot", &config.nats.streams.slot_stream_name, js.clone(), plugin_arc.clone()).await?;
    start_stream_workers("transaction", &config.nats.streams.transaction_stream_name, js.clone(), plugin_arc.clone()).await?;
    start_stream_workers("entry", &config.nats.streams.entry_stream_name, js.clone(), plugin_arc.clone()).await?;
    start_stream_workers("block_metadata", &config.nats.streams.block_metadata_stream_name, js.clone(), plugin_arc.clone()).await?;

    // Wait for Ctrl+C
    signal::ctrl_c().await.expect("failed to listen for ctrl_c");
    match Arc::try_unwrap(plugin_arc) {
            Ok(mut plugin) => plugin.on_unload(),
            Err(_) => {
                eprintln!("Warning: Plugin could not be unwrapped");
            }
        }

    Ok(())
}