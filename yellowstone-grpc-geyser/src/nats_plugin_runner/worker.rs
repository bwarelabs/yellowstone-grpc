use {
    crate::{
        metrics::{
            NATS_BYTES_RECEIVED, NATS_FETCHER_ACTIVE, NATS_MESSAGES_DROPPED, NATS_MESSAGES_FETCHED,
            NATS_QUEUE_DEPTH, NATS_WORKER_DURATION, NATS_WORKER_ERRORS,
        },
        nats_plugin_runner::dispatcher::{
            handle_account, handle_block_metadata, handle_entry, handle_slot, handle_transaction,
        },
        plugin::Plugin,
    },
    async_nats::jetstream::{
        consumer::{pull::OrderedConfig, Consumer},
        Context,
    },
    flume,
    futures::StreamExt,
    log::{error, info, warn},
    std::sync::Arc,
    tokio::{
        select,
        sync::broadcast::Receiver,
        time::{sleep, Duration},
    },
};

pub async fn start_stream_workers(
    label: &str,
    stream_name: &str,
    js: Context,
    plugin: Arc<Plugin>,
    shutdown_rx: Receiver<()>,
) -> anyhow::Result<()> {
    let (tx, rx) = flume::bounded::<Vec<u8>>(5000);

    let stream = js.get_stream(stream_name).await?;
    let consumer: Consumer<OrderedConfig> = stream
        .create_consumer(OrderedConfig {
            name: Some(label.into()),
            max_batch: 64,
            ..Default::default()
        })
        .await?;

    spawn_fetcher_task(
        label.to_string(),
        consumer,
        tx.clone(),
        shutdown_rx.resubscribe(),
    );
    spawn_worker_task(
        label.to_string(),
        plugin,
        rx.clone(),
        shutdown_rx.resubscribe(),
    );
    spawn_metrics_task(label.to_string(), rx.clone(), shutdown_rx.resubscribe());

    Ok(())
}

fn spawn_fetcher_task(
    label: String,
    consumer: Consumer<OrderedConfig>,
    tx: flume::Sender<Vec<u8>>,
    mut shutdown_rx: Receiver<()>,
) {
    tokio::spawn(async move {
        NATS_FETCHER_ACTIVE.with_label_values(&[&label]).set(1);

        let mut messages = match consumer.messages().await {
            Ok(stream) => stream,
            Err(e) => {
                error!(
                    "[{}-fetcher] Failed to start consumer stream: {:?}",
                    label, e
                );
                return;
            }
        };

        loop {
            select! {
                _ = shutdown_rx.recv() => {
                    info!("[{}-fetcher] Shutdown signal received", label);
                    break;
                }
                msg = messages.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            let size = msg.payload.len() as u64;

                            if tx.send_async(msg.payload.to_vec()).await.is_err() {
                                NATS_MESSAGES_DROPPED
                                    .with_label_values(&[&label, "buffer_full"])
                                    .inc();
                                warn!("[{}-fetcher] Dropped message: buffer full", label);
                            }

                            NATS_MESSAGES_FETCHED.with_label_values(&[&label]).inc();
                            NATS_BYTES_RECEIVED
                                .with_label_values(&[&label])
                                .inc_by(size);
                        }
                        Some(Err(e)) => {
                            NATS_WORKER_ERRORS
                                .with_label_values(&[&label, "fetch_error"])
                                .inc();
                            error!("[{}-fetcher] Stream error: {:?}", label, e);
                        }
                        None => {
                            warn!("[{}-fetcher] Message stream ended", label);
                            break;
                        }
                    }
                }
            }
        }

        NATS_FETCHER_ACTIVE.with_label_values(&[&label]).set(0);
    });
}

/// Spawn worker task
///   This is limited to a single worker thread due usage of OrderedConsumer which only supports
///   one consumer. If normal Consumer is used, the bellow code can be used replicated with a
///   for loop and the flume channel will assure that only one worker will be processing a message
fn spawn_worker_task(
    label: String,
    plugin: Arc<Plugin>,
    rx: flume::Receiver<Vec<u8>>,
    mut shutdown_rx: Receiver<()>,
) {
    tokio::spawn(async move {
        loop {
            select! {
                _ = shutdown_rx.recv() => {
                    info!("[{}-worker] Shutdown signal received", label);
                    break;
                }
                result = rx.recv_async() => {
                    match result {
                        Ok(data) => {
                            let timer = NATS_WORKER_DURATION
                                .with_label_values(&[&label])
                                .start_timer();

                            let result = match label.as_str() {
                                "account" => handle_account(&plugin, &data),
                                "slot" => handle_slot(&plugin, &data),
                                "transaction" => handle_transaction(&plugin, &data),
                                "entry" => handle_entry(&plugin, &data),
                                "block_metadata" => handle_block_metadata(&plugin, &data),
                                _ => Err(anyhow::anyhow!("Unknown stream label: {}", label)),
                            };

                            timer.observe_duration();

                            if let Err(e) = result {
                                NATS_WORKER_ERRORS
                                    .with_label_values(&[&label, "handler"])
                                    .inc();
                                error!("[{}-worker] Failed to handle message: {:?}", label, e);
                            }
                        }
                        Err(_) => {
                            warn!("[{}-worker] Channel closed", label);
                            break;
                        }
                    }
                }
            }
        }
    });
}

fn spawn_metrics_task(label: String, rx: flume::Receiver<Vec<u8>>, mut shutdown_rx: Receiver<()>) {
    tokio::spawn(async move {
        loop {
            select! {
                _ = shutdown_rx.recv() => {
                    info!("[{}-monitor] Shutdown signal received", label);
                    break;
                }
                _ = sleep(Duration::from_millis(500)) => {
                    NATS_QUEUE_DEPTH
                        .with_label_values(&[&label])
                        .set(rx.len() as i64);
                }
            }
        }
    });
}
