use crate::ui::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    FileChooser,
    ScriptRunner,
    Unified, // New unified lazygit-style view
}

pub(crate) struct Screen {
    pub mode: Mode,
    pub components: Vec<Box<dyn Component + Send + Sync>>,
}

impl Screen {
    pub fn new(mode: Mode, components: Vec<Box<dyn Component + Send + Sync>>) -> Self {
        Self { mode, components }
    }
}
