use std::collections::HashMap;

use serenity::all::ChannelId;
use tokio::sync::watch;

/// Registry for per-channel cancellation tokens.
/// Allows cleanup tasks to be cancelled when a channel is disabled.
pub struct CancellationRegistry {
    tokens: HashMap<ChannelId, watch::Sender<bool>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Register a new cancellation token for a channel.
    /// Returns a receiver that the cleanup task can watch for cancellation.
    pub fn register(&mut self, channel_id: ChannelId) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        self.tokens.insert(channel_id, tx);
        rx
    }

    /// Signal cancellation for a channel's cleanup task.
    /// Returns true if a task was running and cancelled, false otherwise.
    pub fn cancel(&mut self, channel_id: ChannelId) -> bool {
        if let Some(tx) = self.tokens.get(&channel_id) {
            // Send cancellation signal; ignore error if receiver dropped
            let _ = tx.send(true);
            true
        } else {
            false
        }
    }

    /// Remove a channel's cancellation token.
    pub fn deregister(&mut self, channel_id: ChannelId) {
        self.tokens.remove(&channel_id);
    }

    /// Check if a cleanup task is currently running for a channel.
    pub fn is_running(&self, channel_id: ChannelId) -> bool {
        self.tokens.contains_key(&channel_id)
    }
}
