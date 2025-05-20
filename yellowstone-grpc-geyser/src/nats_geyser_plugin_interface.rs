use {
    agave_geyser_plugin_interface::geyser_plugin_interface::Result,
    solana_nats_geyser_protobufs::{
        account::AccountMessage, block_metadata::BlockMetadataMessage, entry::EntryMessage,
        slot::SlotMessage, transaction::TransactionMessage,
    },
};

pub trait NatsGeyserPlugin: Send + Sync + std::fmt::Debug {
    /// The callback to allow the plugin to setup the logging configuration using the logger
    /// and log level specified by the validator. Will be called first on load/reload, before any other
    /// callback, and only called once.
    /// # Examples
    ///
    /// ```
    /// use agave_geyser_plugin_interface::geyser_plugin_interface::{GeyserPlugin,
    /// GeyserPluginError, Result};
    ///
    /// #[derive(Debug)]
    /// struct SamplePlugin;
    /// impl GeyserPlugin for SamplePlugin {
    ///     fn setup_logger(&self, logger: &'static dyn log::Log, level: log::LevelFilter) -> Result<()> {
    ///        log::set_max_level(level);
    ///        if let Err(err) = log::set_logger(logger) {
    ///            return Err(GeyserPluginError::Custom(Box::new(err)));
    ///        }
    ///        Ok(())
    ///     }
    ///     fn name(&self) -> &'static str {
    ///         &"sample"
    ///     }
    /// }
    /// ```
    #[allow(unused_variables)]
    fn setup_logger(&self, logger: &'static dyn log::Log, level: log::LevelFilter) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &'static str;

    /// The callback called when a plugin is loaded by the system,
    /// used for doing whatever initialization is required by the plugin.
    /// The _config_file contains the name of the
    /// of the config file. The config must be in JSON format and
    /// include a field "libpath" indicating the full path
    /// name of the shared library implementing this interface.
    fn on_load(&mut self, _config_file: &str, _is_reload: bool) -> Result<()> {
        Ok(())
    }
    /// The callback called right before a plugin is unloaded by the system
    /// Used for doing cleanup before unload.
    fn on_unload(&mut self) {}

    // TODO - add missing parameters? also add docs ?
    fn update_account(&self, account: AccountMessage) -> Result<()>;
    fn notify_end_of_startup(&self) -> Result<()>;
    fn update_slot_status(&self, slot: SlotMessage) -> Result<()>;
    fn notify_transaction(&self, transaction: TransactionMessage) -> Result<()>;
    fn notify_entry(&self, entry: EntryMessage) -> Result<()>;
    fn notify_block_metadata(&self, blockinfo: BlockMetadataMessage) -> Result<()>;

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
