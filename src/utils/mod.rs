//! Utility functions for logging, panic handling, etc.

pub mod logging;
pub mod panic;

pub use logging::initialize_logging;
pub use panic::initialize_panic_handler;

// Keep the helper function for now (will be refactored in Step 4)
use crate::action::Action;
use tokio::sync::mpsc::UnboundedSender;

pub fn send_through_channel(channel: &Option<UnboundedSender<Action>>, action: Action) {
    if let Some(channel) = channel {
        if let Err(error) = channel.send(action) {
            log::error!("{}", error);
        }
    }
}
