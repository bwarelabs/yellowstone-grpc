use dashmap::DashMap;
use tokio::sync::broadcast;
use std::sync::{
    Arc,
    atomic::{
        AtomicUsize, Ordering
    }
};

#[derive(Debug, Clone)]
pub struct ConnectionManager {
    connections: Arc<DashMap<String, Arc<ConnectionHandle>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, team_id: String, shutdown_tx: broadcast::Sender<()>) {
        self.connections
            .entry(team_id)
            .and_modify(|handle| handle.inc())
            .or_insert_with(|| Arc::new(ConnectionHandle::new(shutdown_tx)));
    }

    pub fn unregister(&self, team_id: &str) {
        if let Some(entry) = self.connections.get_mut(team_id) {
            let count = entry.dec();
            if count == 1 {
                self.connections.remove(team_id);
            }
        }
    }

    pub fn shutdown_client(&self, team_id: &str) {
        if let Some(handle) = self.connections.get(team_id) {
            let _ = handle.shutdown_tx.send(());
        }
    }

    pub fn list_active_teams(&self) -> Vec<String> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }
}


#[derive(Debug)]
pub struct ConnectionHandle {
    pub shutdown_tx: broadcast::Sender<()>,
    pub ref_count: AtomicUsize,
}

impl ConnectionHandle {
    pub fn new(shutdown_tx: broadcast::Sender<()>) -> Self {
        Self {
            shutdown_tx,
            ref_count: AtomicUsize::new(1),
        }
    }

    pub fn inc(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Relaxed)
    }

    pub fn count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
}

