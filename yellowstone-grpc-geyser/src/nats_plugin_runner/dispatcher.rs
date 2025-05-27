use {
    crate::{nats_geyser_plugin_interface::NatsGeyserPlugin, plugin::Plugin},
    anyhow::{Context, Result},
    bincode::{config, decode_from_slice},
    solana_nats_geyser_protobufs::{
        account::AccountMessage, block_metadata::BlockMetadataMessage, entry::EntryMessage,
        slot::SlotMessage, transaction::TransactionMessage,
    },
    std::sync::Arc,
};

pub fn handle_account(plugin: &Arc<Plugin>, data: &[u8]) -> Result<()> {
    let (msg, _): (AccountMessage, usize) = decode_from_slice(data, config::standard())
        .context("Failed to deserialize AccountMessage")?;
    plugin
        .update_account(msg)
        .context("Plugin::update_account failed")
}

pub fn handle_slot(plugin: &Arc<Plugin>, data: &[u8]) -> Result<()> {
    let (msg, _): (SlotMessage, usize) =
        decode_from_slice(data, config::standard()).context("Failed to deserialize SlotMessage")?;
    plugin
        .update_slot_status(msg)
        .context("Plugin::update_slot_status failed")
}

pub fn handle_transaction(plugin: &Arc<Plugin>, data: &[u8]) -> Result<()> {
    let (msg, _): (TransactionMessage, usize) = decode_from_slice(data, config::standard())
        .context("Failed to deserialize TransactionMessage")?;
    plugin
        .notify_transaction(msg)
        .context("Plugin::notify_transaction failed")
}

pub fn handle_entry(plugin: &Arc<Plugin>, data: &[u8]) -> Result<()> {
    let (msg, _): (EntryMessage, usize) = decode_from_slice(data, config::standard())
        .context("Failed to deserialize EntryMessage")?;
    plugin
        .notify_entry(msg)
        .context("Plugin::notify_entry failed")
}

pub fn handle_block_metadata(plugin: &Arc<Plugin>, data: &[u8]) -> Result<()> {
    let (msg, _): (BlockMetadataMessage, usize) = decode_from_slice(data, config::standard())
        .context("Failed to deserialize BlockMetadataMessage")?;
    plugin
        .notify_block_metadata(msg)
        .context("Plugin::notify_block_metadata failed")
}
