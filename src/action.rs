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
    ScriptRun,
    Suspend,
    Resume,
    Quit,
    Refresh,
    Error(String),
    Help,
}
