use crate::{app::Script, entries::EntryStatus};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Main actions
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    Refresh,
    Error(String),

    // Cursor actions
    CursorUp,
    CursorDown,
    CursorToTop,
    CursorToBottom,
    JumpToNextNotRun, // Jump to next Not Run script (skipping Success/Error/Skipped)
    JumpToPath(crate::domain::ScriptPath, usize), // Jump to a specific path (path, retry_count)
    SelectFromCursorToEnd, // Select all scripts from cursor to end

    // Directory actions
    DirectoryOpenSelected,
    DirectoryExpand,   // Right arrow: expand directory (open only, don't toggle)
    DirectoryCollapse, // Left arrow: collapse directory or go to parent

    // Async actions
    ScriptRun(bool),
    ScriptRunning(crate::domain::ScriptPath),
    ScriptFinished(crate::domain::ScriptPath, u128, u32),
    ScriptError(crate::domain::ScriptPath, String, Option<u32>),
    ClearOutput, // Clear execution output
    CalculateEntryStatus,
    CheckForChanges, // Check for file modifications (CRC check)
    EntryStatusChanged(crate::domain::ScriptPath, EntryStatus),

    // Async loading actions
    EntriesLoading, // Signal that entries are being loaded
    EntriesLoaded(Vec<crate::entries::ListEntry>), // Entries loaded from async task
    DirectoryChildrenLoaded(crate::domain::ScriptPath, Vec<crate::entries::ListEntry>), // (parent_path, children) loaded from async task
    StatusCalculationProgress(usize, usize), // (current, total) for CRC calculation progress
    SearchingForNextNotRun(bool),            // (is_searching) - show/hide searching indicator

    // Selection actions
    SelectCurrent,
    UnselectAll,
    ToggleSkip, // Toggle skip status on current script/folder
    AddSelection(Vec<crate::domain::ScriptPath>),
    RemoveSelection(Vec<crate::domain::ScriptPath>),
    ToggleSelection(Vec<crate::domain::ScriptPath>),
    SelectionChanged(Vec<crate::domain::ScriptPath>),
    ScriptHighlighted(Option<Script>),
    FileContentLoaded(crate::domain::ScriptPath, Vec<String>, usize, u64), // (path, preview_lines, line_count, file_size)

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
