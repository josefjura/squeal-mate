use crate::{app::Script, entries::EntryStatus, screen::Mode};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Main actions
    Tick,
    Render,
    Resize(u16, u16),
    SwitchMode(Mode),
    Suspend,
    Resume,
    Quit,
    Refresh,
    Error(String),
    Help,

    // Cursor actions
    CursorUp,
    CursorDown,
    CursorToTop,
    CursorToBottom,

    // Directory actions
    DirectoryOpenSelected,
    DirectoryLeave,

    // Help
    ToggleHelp,
    CloseHelp,

    // Async actions
    ScriptRun(bool),
    ScriptRunning(String),
    ScriptFinished(String, u128, u32),
    ScriptError(String, String, Option<u32>),
    CalculateEntryStatus,
    EntryStatusChanged(String, EntryStatus),

    // Selection actions
    SelectCurrent,
    SelectAllAfter,
    SelectAllAfterInDirectory,
    SelectAllInDirectory,
    UnselectAll,
    UnselectCurrent,
    AddSelection(Vec<String>),
    RemoveSelection(Vec<String>),
    ToggleSelection(Vec<String>),
    SelectionChanged(Vec<String>),
    ScriptHighlighted(Option<Script>),
}
