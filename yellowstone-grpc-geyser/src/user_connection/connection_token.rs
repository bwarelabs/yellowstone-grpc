use {
    crate::user_connection::connection_manager::ConnectionManager, log::info, std::sync::Arc,
    tokio::sync::watch,
};

#[derive(Clone)]
pub struct ConnectionToken {
    team_id: String,
    manager: Arc<ConnectionManager>,
    sender: watch::Sender<bool>,
    receiver: Option<watch::Receiver<bool>>,
}

impl ConnectionToken {
    pub fn new(
        team_id: String,
        manager: Arc<ConnectionManager>,
        sender: watch::Sender<bool>,
        receiver: watch::Receiver<bool>,
    ) -> Self {
        Self {
            team_id,
            manager,
            sender,
            receiver: Some(receiver),
        }
    }

    pub fn shutdown_rx(&mut self) -> &mut watch::Receiver<bool> {
        self.receiver.as_mut().expect("Receiver already taken")
    }
}

impl Drop for ConnectionToken {
    fn drop(&mut self) {
        // If only one receiver left (i.e. this one), remove from DashMap
        if self.sender.receiver_count() <= 1 {
            info!("Cleaning up entry for team_id: {}", self.team_id);
            self.manager.connections.remove(&self.team_id);
        }
    }
}
