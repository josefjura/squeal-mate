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
    JumpToNextNotRun,  // Jump to next Not Run script (skipping Success/Error/Skipped)
    JumpToPath(String, usize),  // Jump to a specific path (path, retry_count)
    SelectFromCursorToEnd,  // Select all scripts from cursor to end

    // Directory actions
    DirectoryOpenSelected,
    DirectoryExpand,   // Right arrow: expand directory (open only, don't toggle)
    DirectoryCollapse, // Left arrow: collapse directory or go to parent

    // Help
    ToggleHelp,
    CloseHelp,

    // Async actions
    ScriptRun(bool),
    ScriptRunning(String),
    ScriptFinished(String, u128, u32),
    ScriptError(String, String, Option<u32>),
    ClearOutput,  // Clear execution output
    CalculateEntryStatus,
    CheckForChanges,  // Check for file modifications (CRC check)
    EntryStatusChanged(String, EntryStatus),

    // Async loading actions
    EntriesLoading,  // Signal that entries are being loaded
    EntriesLoaded(Vec<crate::entries::ListEntry>),  // Entries loaded from async task
    DirectoryChildrenLoaded(String, Vec<crate::entries::ListEntry>),  // (parent_path, children) loaded from async task
    StatusCalculationProgress(usize, usize),  // (current, total) for CRC calculation progress
    SearchingForNextNotRun(bool),  // (is_searching) - show/hide searching indicator

    // Selection actions
    SelectCurrent,
    UnselectAll,
    ToggleSkip,  // Toggle skip status on current script/folder
    AddSelection(Vec<String>),
    RemoveSelection(Vec<String>),
    ToggleSelection(Vec<String>),
    SelectionChanged(Vec<String>),
    ScriptHighlighted(Option<Script>),
    FileContentLoaded(String, Vec<String>, usize, u64),  // (path, preview_lines, line_count, file_size)

    // Panel navigation (for unified view)
    FocusNextPanel,
    FocusPreviousPanel,
    PanelFocusChanged(PanelFocus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    FileTree,
    ScriptPreview,
    ExecutionLog,
}
