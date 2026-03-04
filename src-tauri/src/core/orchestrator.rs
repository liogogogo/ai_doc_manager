use std::sync::Arc;
use crate::services::db::Database;

/// The Orchestrator coordinates all core features and manages their lifecycle.
/// It schedules periodic tasks (GC, conflict detection) and responds to
/// file-change events from the FileWatcher.
pub struct Orchestrator {
    db: Arc<Database>,
}

impl Orchestrator {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Start background tasks: file watching, scheduled GC, etc.
    pub async fn start(&self) {
        tracing::info!("Orchestrator started");
        // TODO: Spawn background tasks
        // - File watcher loop
        // - Scheduled GC timer
        // - Scheduled conflict scan timer
    }

    /// Stop all background tasks gracefully
    pub async fn stop(&self) {
        tracing::info!("Orchestrator stopping");
    }
}
