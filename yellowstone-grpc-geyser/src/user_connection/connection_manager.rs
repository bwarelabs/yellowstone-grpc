use {
  std::sync::Arc,
  dashmap::{
      DashMap,
      mapref::entry::Entry,
  },
    tokio::sync::watch,
  crate::{
      user_connection::connection_token::ConnectionToken
  },
};

#[derive(Debug, Clone)]

pub struct ConnectionManager {
    pub connections: Arc<DashMap<String, watch::Sender<bool>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Registers a new or existing connection and returns a token
    /// that cleans up automatically when dropped.
    pub fn register_team(self: Arc<Self>, team_id: String) -> ConnectionToken {
        let sender = match self.connections.entry(team_id.clone()) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                let (tx, _) = watch::channel(false); // false = not shutdown
                e.insert(tx.clone());
                tx
            }
        };

        let receiver = sender.subscribe();
        ConnectionToken::new(team_id, self.clone(), sender, receiver)
    }

    /// Sends a shutdown signal to all listeners of this team
    pub fn shutdown_client(&self, team_id: &str) {
        if let Some(sender) = self.connections.get(team_id) {
            let _ = sender.send(true); // true = shutdown
        }
    }

    pub fn list_active_teams(&self) -> Vec<String> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }
}
