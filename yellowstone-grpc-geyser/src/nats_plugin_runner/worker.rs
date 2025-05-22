use {
    crate::{
        metrics::{
            NATS_FETCHER_ACTIVE, NATS_MESSAGES_DROPPED, NATS_MESSAGES_FETCHED,
            NATS_WORKER_DURATION, NATS_WORKER_ERRORS,
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
    tokio::time::Duration,
};

pub async fn start_stream_workers(
    label: &str,
    stream_name: &str,
    js: Context,
    plugin: Arc<Plugin>,
) -> anyhow::Result<()> {
    let (tx, rx) = flume::bounded::<Vec<u8>>(5000);
    let num_workers = 1;

    // Create ephemeral ordered consumer
    let stream = js.get_stream(stream_name).await?;
    let consumer: Consumer<OrderedConfig> = stream
        .create_consumer(OrderedConfig {
            max_batch: 64,
            // max_expires: Duration::from_secs(2),
            ..Default::default()
        })
        .await?;

    // Spawn fetcher task
    {
        let tx = tx.clone();
        let label = label.to_string();
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

            while let Some(message_result) = messages.next().await {
                match message_result {
                    Ok(msg) => {
                        NATS_MESSAGES_FETCHED.with_label_values(&[&label]).inc();

                        if tx.send_async(msg.payload.to_vec()).await.is_err() {
                            NATS_MESSAGES_DROPPED
                                .with_label_values(&[&label, "buffer_full"])
                                .inc();
                            warn!("[{}-fetcher] Dropped message: buffer full", label);
                        }
                    }
                    Err(e) => {
                        NATS_WORKER_ERRORS
                            .with_label_values(&[&label, "fetch_error"])
                            .inc();
                        error!("[{}-fetcher] Stream error: {:?}", label, e);
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
            }

            NATS_FETCHER_ACTIVE.with_label_values(&[&label]).set(0);
        });
    }

    // Spawn worker tasks
    for i in 0..num_workers {
        let plugin = plugin.clone();
        let rx = rx.clone();
        let label = label.to_string();
        let mut no_of_messages = 0;

        tokio::spawn(async move {
            while let Ok(data) = rx.recv_async().await {
                no_of_messages += 1;
                // info!("thread-{} received message", i);
                let timer = NATS_WORKER_DURATION
                    .with_label_values(&[&label])
                    .start_timer();

                let result = match label.as_str() {
                    "account" => handle_account(&plugin, &data, i),
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
                    error!("[{}-worker-{}] Failed to handle message: {:?}", label, i, e);
                }

                info!(
                    "[{}-worker-{}] Processed {} messages",
                    label, i, no_of_messages
                );
            }
        });
    }

    Ok(())
}
