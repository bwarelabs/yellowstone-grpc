use dashmap::DashMap;
use tokio::sync::broadcast;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ConnectionManager {
    connections: Arc<DashMap<String, broadcast::Sender<()>>>, // team_id -> shutdown sender
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub fn get_existing_sender(&self, team_id: &str) -> Option<broadcast::Sender<()>> {
        self.connections.get(team_id).map(|s| s.clone())
    }

    pub fn register(&self, team_id: String, shutdown_tx: broadcast::Sender<()>) {
        self.connections.entry(team_id).or_insert(shutdown_tx);
    }

    pub fn unregister(&self, team_id: &str) {
        self.connections.remove(team_id);
    }

    pub fn shutdown_client(&self, team_id: &str) {
        if let Some(handle) = self.connections.get(team_id) {
            let _ = handle.send(());
        }
    }

    pub fn list_active_teams(&self) -> Vec<String> {
        self.connections.iter().map(|entry| entry.key().clone()).collect()
    }
}
