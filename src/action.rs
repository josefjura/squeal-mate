use std::path::PathBuf;

use crate::app::{MessageType, Mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    CursorUp,
    CursorDown,
    CursorToTop,
    CursorToBottom,
    DirectoryOpenSelected,
    DirectoryLeave,
    Message(String, MessageType),
    SelectCurrent,
    SelectAllAfter,
    SelectScripts(Vec<PathBuf>),
    AppendScripts(Vec<PathBuf>),
    RunScripts,
    SwitchMode(Mode),
    StartSpinner,
    StopSpinner,
    ScriptRun,
    Suspend,
    Resume,
    Quit,
    Refresh,
    Error(String),
    Help,
}
