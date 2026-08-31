//! Centralized action dispatching
//!
//! Replaces manual tokio::spawn and channel sends throughout the codebase

use crate::action::Action;
use tokio::sync::mpsc::UnboundedSender;

/// Centralized dispatcher for actions
///
/// This replaces the pattern of manually spawning tasks and sending actions
/// through channels everywhere in the codebase.
#[derive(Clone)]
pub struct ActionDispatcher {
    tx: UnboundedSender<Action>,
}

impl ActionDispatcher {
    /// Create a new dispatcher
    pub fn new(tx: UnboundedSender<Action>) -> Self {
        Self { tx }
    }

    /// Dispatch an action immediately
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.tx.send(action) {
            log::error!("Failed to dispatch action: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_dispatch() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = ActionDispatcher::new(tx);

        dispatcher.dispatch(Action::Tick);

        let received = rx.recv().await;
        assert!(matches!(received, Some(Action::Tick)));
    }
}
