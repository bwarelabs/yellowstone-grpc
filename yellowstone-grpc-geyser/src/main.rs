use {
    async_nats::{connect, jetstream},
    std::{process::exit, sync::Arc},
    tokio::{signal, sync::broadcast},
    yellowstone_grpc_geyser::{
        nats_geyser_plugin_interface::NatsGeyserPlugin,
        nats_plugin_runner::{
            config::CONFIG, constants::WorkerLabels, worker::start_stream_workers,
        },
        plugin::Plugin,
    },
};

fn main() -> anyhow::Result<()> {
    let mut plugin = Plugin::default();
    plugin.on_load("config.json", false)?;

    let plugin_arc = Arc::new(plugin);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main(plugin_arc.clone()))
}

async fn async_main(plugin_arc: Arc<Plugin>) -> anyhow::Result<()> {
    let client = connect(&CONFIG.nats.url).await?;
    let js = jetstream::new(client);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    start_stream_workers(
        WorkerLabels::ACCOUNT,
        &CONFIG.nats.streams.name.account,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        WorkerLabels::SLOT,
        &CONFIG.nats.streams.name.slot,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        WorkerLabels::TRANSACTION,
        &CONFIG.nats.streams.name.transaction,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        WorkerLabels::ENTRY,
        &CONFIG.nats.streams.name.entry,
        js.clone(),
        plugin_arc.clone(),
        shutdown_rx.resubscribe(),
    )
    .await?;
    start_stream_workers(
        WorkerLabels::BLOCK_METADATA,
        &CONFIG.nats.streams.name.block_metadata,
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
