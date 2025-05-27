use {
    async_nats::{connect, jetstream},
    std::{process::exit, sync::Arc},
    tokio::{signal, sync::broadcast},
    yellowstone_grpc_geyser::{
        nats_geyser_plugin_interface::NatsGeyserPlugin,
        nats_plugin_runner::{
            config::{load_config, Config},
            worker::start_stream_workers,
        },
        plugin::Plugin,
    },
};

fn main() -> anyhow::Result<()> {
    let config = load_config("nats_config.toml")?;

    // Create and load plugin before entering the async context
    let mut plugin = Plugin::default();
    plugin.on_load("config.json", false)?;

    let plugin_arc = Arc::new(plugin);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main(&config, plugin_arc.clone()))
}

async fn async_main(config: &Config, plugin_arc: Arc<Plugin>) -> anyhow::Result<()> {
    let client = connect(&config.nats.url).await?;
    let js = jetstream::new(client);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // TODO: move labels to some consts
    start_stream_workers(
        "account",
        &config.nats.streams.account_stream_name,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        "slot",
        &config.nats.streams.slot_stream_name,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        "transaction",
        &config.nats.streams.transaction_stream_name,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        "entry",
        &config.nats.streams.entry_stream_name,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        "block_metadata",
        &config.nats.streams.block_metadata_stream_name,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx,
    )
    .await?;

    signal::ctrl_c().await?;
    println!("Ctrl+C received, sending shutdown signal...");

    let _ = shutdown_tx.send(());

    // TODO: Fix this dirty exit. I was not able to gracefully stop all the runtimes so the main loop can exit by itself
    exit(0);
}
