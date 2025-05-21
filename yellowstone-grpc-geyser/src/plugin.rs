#[allow(unused_imports)]
use {
    crate::{
        config::Config,
        grpc::GrpcService,
        metrics::{self, PrometheusService},
        nats_geyser_plugin_interface::NatsGeyserPlugin,
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin, GeyserPluginError, Result as PluginResult, SlotStatus as GeyserSlotStatus,
    },
    solana_nats_geyser_protobufs::{
        account::AccountMessage as NatsAccountMessage,
        block_metadata::BlockMetadataMessage as NatsBlockMetadataMessage,
        entry::EntryMessage as NatsEntryMessage, slot::SlotMessage as NatsSlotMessage,
        transaction::TransactionMessage as NatsTransactionMessage,
    },
    std::{
        concat, env,
        sync::{atomic::AtomicBool, Arc, Mutex},
        time::Duration,
    },
    tokio::{
        runtime::{Builder, Runtime},
        sync::{mpsc, Notify},
    },
    yellowstone_grpc_proto::plugin::message::{
        Message, MessageAccount, MessageBlockMeta, MessageEntry, MessageSlot, MessageTransaction,
    },
};

#[derive(Debug)]
pub struct PluginInner {
    runtime: Runtime,
    snapshot_channel: Mutex<Option<crossbeam_channel::Sender<Box<Message>>>>,
    #[allow(dead_code)]
    snapshot_channel_closed: AtomicBool,
    grpc_channel: mpsc::UnboundedSender<Message>,
    grpc_shutdown: Arc<Notify>,
    prometheus: PrometheusService,
}

impl PluginInner {
    fn send_message(&self, message: Message) {
        if self.grpc_channel.send(message).is_ok() {
            metrics::message_queue_size_inc();
        }
    }
}

#[derive(Debug, Default)]
pub struct Plugin {
    inner: Option<PluginInner>,
}

impl Plugin {
    fn with_inner<F>(&self, f: F) -> PluginResult<()>
    where
        F: FnOnce(&PluginInner) -> PluginResult<()>,
    {
        let inner = self.inner.as_ref().expect("initialized");
        f(inner)
    }
}

impl NatsGeyserPlugin for Plugin {
    fn name(&self) -> &'static str {
        concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"))
    }

    fn on_load(&mut self, config_file: &str, is_reload: bool) -> PluginResult<()> {
        let config = Config::load_from_file(config_file)?;

        // Setup logger
        solana_logger::setup_with_default(&config.log.level);

        // Create inner
        let mut builder = Builder::new_multi_thread();
        if let Some(worker_threads) = config.tokio.worker_threads {
            builder.worker_threads(worker_threads);
        }
        if let Some(tokio_cpus) = config.tokio.affinity.clone() {
            builder.on_thread_start(move || {
                affinity::set_thread_affinity(&tokio_cpus).expect("failed to set affinity")
            });
        }
        let runtime = builder
            .thread_name_fn(crate::get_thread_name)
            .enable_all()
            .build()
            .map_err(|error| GeyserPluginError::Custom(Box::new(error)))?;

        let (snapshot_channel, grpc_channel, grpc_shutdown, prometheus) =
            runtime.block_on(async move {
                let (debug_client_tx, debug_client_rx) = mpsc::unbounded_channel();
                let (snapshot_channel, grpc_channel, grpc_shutdown) = GrpcService::create(
                    config.tokio,
                    config.grpc,
                    config.debug_clients_http.then_some(debug_client_tx),
                    is_reload,
                )
                .await
                .map_err(|error| GeyserPluginError::Custom(format!("{error:?}").into()))?;
                let prometheus = PrometheusService::new(
                    config.prometheus,
                    config.debug_clients_http.then_some(debug_client_rx),
                )
                .await
                .map_err(|error| GeyserPluginError::Custom(Box::new(error)))?;
                Ok::<_, GeyserPluginError>((
                    snapshot_channel,
                    grpc_channel,
                    grpc_shutdown,
                    prometheus,
                ))
            })?;

        self.inner = Some(PluginInner {
            runtime,
            snapshot_channel: Mutex::new(snapshot_channel),
            snapshot_channel_closed: AtomicBool::new(false),
            grpc_channel,
            grpc_shutdown,
            prometheus,
        });

        Ok(())
    }

    fn on_unload(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.grpc_shutdown.notify_one();
            drop(inner.grpc_channel);
            inner.prometheus.shutdown();
            inner.runtime.shutdown_timeout(Duration::from_secs(30));
        }
    }

    fn update_account(&self, account: NatsAccountMessage) -> PluginResult<()> {
        self.with_inner(|inner| {
            let slot = account.slot;
            let message = Message::Account(MessageAccount::from_nats(account, slot, false));
            inner.send_message(message);

            Ok(())
        })
    }

    fn notify_end_of_startup(&self) -> PluginResult<()> {
        self.with_inner(|inner| {
            let _snapshot_channel = inner.snapshot_channel.lock().unwrap().take();
            Ok(())
        })
    }

    fn update_slot_status(&self, slot: NatsSlotMessage) -> PluginResult<()> {
        self.with_inner(|inner| {
            let NatsSlotMessage {
                slot,
                parent,
                status,
            } = slot;
            let geyser_slot_status = GeyserSlotStatus::from(status);
            let message =
                Message::Slot(MessageSlot::from_geyser(slot, parent, &geyser_slot_status));
            inner.send_message(message);
            metrics::update_slot_status(&geyser_slot_status, slot);
            Ok(())
        })
    }

    fn notify_transaction(&self, transaction: NatsTransactionMessage) -> PluginResult<()> {
        let slot = transaction.slot;
        self.with_inner(|inner| {
            let message = Message::Transaction(MessageTransaction::from_nats(transaction, slot));
            inner.send_message(message);

            Ok(())
        })
    }

    fn notify_entry(&self, entry: NatsEntryMessage) -> PluginResult<()> {
        self.with_inner(|inner| {
            let message = Message::Entry(Arc::new(MessageEntry::from_nats(entry)));
            inner.send_message(message);

            Ok(())
        })
    }

    fn notify_block_metadata(&self, blockinfo: NatsBlockMetadataMessage) -> PluginResult<()> {
        self.with_inner(|inner| {
            let message = Message::BlockMeta(Arc::new(MessageBlockMeta::from_nats(blockinfo)));
            inner.send_message(message);

            Ok(())
        })
    }

    fn account_data_notifications_enabled(&self) -> bool {
        true
    }

    fn account_data_snapshot_notifications_enabled(&self) -> bool {
        false
    }

    fn transaction_notifications_enabled(&self) -> bool {
        true
    }

    fn entry_notifications_enabled(&self) -> bool {
        true
    }
}

// #[no_mangle]
// #[allow(improper_ctypes_definitions)]
// /// # Safety
// ///
// /// This function returns the Plugin pointer as trait GeyserPlugin.
// pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
//     let plugin = Plugin::default();
//     let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
//     Box::into_raw(plugin)
// }
