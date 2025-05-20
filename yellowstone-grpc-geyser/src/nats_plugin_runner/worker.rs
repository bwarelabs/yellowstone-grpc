use {
    crate::plugin::Plugin,
    async_nats::jetstream::{
        consumer::{pull::OrderedConfig, OrderedPullConsumer},
        Context,
    },
    flume,
    futures::StreamExt,
    std::sync::Arc,
};

pub async fn start_stream_workers(
    label: &str,
    stream_name: &str,
    js: Context,
    plugin: Arc<Plugin>,
) -> anyhow::Result<()> {
    use crate::nats_plugin_runner::dispatcher::*;

    let (tx, rx) = flume::bounded::<Vec<u8>>(5000);
    let num_workers = 4;

    // Create the ephemeral ordered consumer (no durable name allowed)
    let stream = js.get_stream(stream_name).await?;
    let consumer: OrderedPullConsumer = stream
        .create_consumer(OrderedConfig {
            max_batch: 64,
            max_expires: std::time::Duration::from_secs(2),
            ..Default::default()
        })
        .await?;

    // Spawn fetcher task
    {
        let tx = tx.clone();
        let label = label.to_string();

        tokio::spawn(async move {
            let mut messages = consumer
                .messages()
                .await
                .expect("Failed to start consumer stream");

            while let Some(message_result) = messages.next().await {
                match message_result {
                    Ok(msg) => {
                        if let Err(_) = tx.send_async(msg.payload.to_vec()).await {
                            log::warn!("[{}-fetcher] Dropped message: buffer full", label);
                        }
                    }
                    Err(e) => {
                        log::error!("[{}-fetcher] Stream error: {:?}", label, e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    }
                }
            }
        });
    }

    // Spawn worker tasks
    for i in 0..num_workers {
        let plugin = plugin.clone();
        let rx = rx.clone();
        let label = label.to_string();

        tokio::spawn(async move {
            while let Ok(data) = rx.recv_async().await {
                let result = match label.as_str() {
                    "account" => handle_account(&plugin, &data),
                    "slot" => handle_slot(&plugin, &data),
                    "transaction" => handle_transaction(&plugin, &data),
                    "entry" => handle_entry(&plugin, &data),
                    "block_metadata" => handle_block_metadata(&plugin, &data),
                    _ => Err(anyhow::anyhow!("Unknown stream label: {}", label)),
                };

                if let Err(e) = result {
                    log::error!("[{}-worker-{}] Failed to handle message: {:?}", label, i, e);
                }
            }
        });
    }

    Ok(())
}
