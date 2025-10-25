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

    /// Spawn an async task that will dispatch an action when complete
    pub fn dispatch_async<F>(&self, future: F)
    where
        F: std::future::Future<Output = Action> + Send + 'static,
    {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let action = future.await;
            if let Err(e) = tx.send(action) {
                log::error!("Failed to dispatch async action: {}", e);
            }
        });
    }

    /// Spawn an async task that may dispatch multiple actions
    pub fn dispatch_task<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(task);
    }

    /// Get a clone of the underlying sender (for backwards compatibility)
    pub fn sender(&self) -> UnboundedSender<Action> {
        self.tx.clone()
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

    #[tokio::test]
    async fn test_dispatch_async() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = ActionDispatcher::new(tx);

        dispatcher.dispatch_async(async { Action::Tick });

        // Give the async task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let received = rx.recv().await;
        assert!(matches!(received, Some(Action::Tick)));
    }
}
