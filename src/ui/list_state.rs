//! State management for the List component
//!
//! This module consolidates all component state and provides reducer-style
//! state transition methods that are easy to test and reason about.

use std::path::PathBuf;
use crate::entries::{EntryStatus, ListEntry};

/// Consolidated state for the List component
/// All state updates happen through reducer methods
#[derive(Debug, Clone)]
pub struct ComponentState {
    /// Current directory being displayed
    pub current_directory: PathBuf,

    /// Entries in the current directory
    pub entries: Vec<ListEntry>,

    /// Current cursor position
    pub cursor: usize,

    /// Loading state
    pub loading: bool,

    /// CRC calculation progress (current, total)
    pub crc_progress: Option<(usize, usize)>,

    /// Name to position cursor on after loading completes
    pub pending_cursor_name: Option<String>,
}

impl ComponentState {
    /// Create new state with initial directory
    pub fn new(initial_directory: PathBuf) -> Self {
        Self {
            current_directory: initial_directory,
            entries: Vec::new(),
            cursor: 0,
            loading: false,
            crc_progress: None,
            pending_cursor_name: None,
        }
    }

    // ===== Reducer Methods =====
    // Pure state transition functions

    /// Start loading entries - clear entries and show loading state
    pub fn start_loading(&mut self) {
        self.loading = true;
        self.entries = vec![ListEntry {
            name: "Loading...".to_string(),
            relative_path: "".to_string(),
            selected: false,
            is_directory: false,
            status: EntryStatus::Loading,
        }];
    }

    /// Entries have been loaded - update entries and position cursor
    pub fn on_entries_loaded(&mut self, entries: Vec<ListEntry>) {
        self.entries = entries;
        self.loading = false;

        // Position cursor on pending name if set
        if let Some(name) = self.pending_cursor_name.take() {
            self.cursor = self.find_entry_index(&name).unwrap_or(0);
        } else {
            // Default to first entry
            self.cursor = if !self.entries.is_empty() { 0 } else { 0 };
        }
    }

    /// Navigate into a directory - prepare state for loading
    pub fn on_navigation_down(&mut self, new_directory: PathBuf) {
        self.current_directory = new_directory;
        self.pending_cursor_name = None;
        self.start_loading();
    }

    /// Navigate up to parent directory - remember current directory name for cursor positioning
    pub fn on_navigation_up(&mut self) -> Option<String> {
        // Get current directory name to position cursor on
        let old_name = self.current_directory
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Move to parent
        if let Some(parent) = self.current_directory.parent() {
            self.current_directory = parent.to_path_buf();
            self.pending_cursor_name = old_name.clone();
            self.start_loading();
        }

        old_name
    }

    /// Update CRC calculation progress
    pub fn on_crc_progress(&mut self, current: usize, total: usize) {
        if current >= total {
            // Complete - hide progress
            self.crc_progress = None;
        } else {
            self.crc_progress = Some((current, total));
        }
    }

    /// Update status for a specific entry
    pub fn update_entry_status(&mut self, relative_path: &str, status: EntryStatus) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.relative_path == relative_path) {
            entry.status = status;
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor < self.entries.len().saturating_sub(1) {
            self.cursor = self.cursor.saturating_add(1);
        }
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&ListEntry> {
        self.entries.get(self.cursor)
    }

    /// Get mutable reference to currently selected entry
    pub fn selected_entry_mut(&mut self) -> Option<&mut ListEntry> {
        self.entries.get_mut(self.cursor)
    }

    // ===== Helper Methods =====

    /// Find the index of an entry by name
    fn find_entry_index(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.name == name)
    }

    /// Get progress percentage (0-100)
    pub fn progress_percentage(&self) -> Option<u8> {
        self.crc_progress.map(|(current, total)| {
            if total == 0 {
                100
            } else {
                ((current as f64 / total as f64) * 100.0) as u8
            }
        })
    }

    /// Check if we're currently loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Check if we have CRC calculation in progress
    pub fn has_crc_progress(&self) -> bool {
        self.crc_progress.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = ComponentState::new(PathBuf::from("/test"));
        assert_eq!(state.current_directory, PathBuf::from("/test"));
        assert_eq!(state.entries.len(), 0);
        assert_eq!(state.cursor, 0);
        assert!(!state.loading);
        assert!(state.crc_progress.is_none());
    }

    #[test]
    fn test_start_loading() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        state.start_loading();

        assert!(state.loading);
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].name, "Loading...");
    }

    #[test]
    fn test_entries_loaded_without_pending_cursor() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        state.start_loading();

        let entries = vec![
            ListEntry {
                name: "file1.sql".to_string(),
                relative_path: "file1.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
            ListEntry {
                name: "file2.sql".to_string(),
                relative_path: "file2.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
        ];

        state.on_entries_loaded(entries);

        assert!(!state.loading);
        assert_eq!(state.entries.len(), 2);
        assert_eq!(state.cursor, 0); // Default to first entry
    }

    #[test]
    fn test_entries_loaded_with_pending_cursor() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        state.pending_cursor_name = Some("file2.sql".to_string());

        let entries = vec![
            ListEntry {
                name: "file1.sql".to_string(),
                relative_path: "file1.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
            ListEntry {
                name: "file2.sql".to_string(),
                relative_path: "file2.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
        ];

        state.on_entries_loaded(entries);

        assert_eq!(state.cursor, 1); // Should position on file2.sql
        assert!(state.pending_cursor_name.is_none()); // Should be consumed
    }

    #[test]
    fn test_navigation_up() {
        let mut state = ComponentState::new(PathBuf::from("/test/subdir"));

        let old_name = state.on_navigation_up();

        assert_eq!(old_name, Some("subdir".to_string()));
        assert_eq!(state.current_directory, PathBuf::from("/test"));
        assert_eq!(state.pending_cursor_name, Some("subdir".to_string()));
        assert!(state.loading);
    }

    #[test]
    fn test_crc_progress() {
        let mut state = ComponentState::new(PathBuf::from("/test"));

        state.on_crc_progress(5, 10);
        assert_eq!(state.crc_progress, Some((5, 10)));
        assert_eq!(state.progress_percentage(), Some(50));

        state.on_crc_progress(10, 10);
        assert!(state.crc_progress.is_none()); // Complete - should be hidden
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        state.entries = vec![
            ListEntry {
                name: "file1.sql".to_string(),
                relative_path: "file1.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
            ListEntry {
                name: "file2.sql".to_string(),
                relative_path: "file2.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
            ListEntry {
                name: "file3.sql".to_string(),
                relative_path: "file3.sql".to_string(),
                selected: false,
                is_directory: false,
                status: EntryStatus::Unknown,
            },
        ];

        assert_eq!(state.cursor, 0);

        state.cursor_down();
        assert_eq!(state.cursor, 1);

        state.cursor_down();
        assert_eq!(state.cursor, 2);

        state.cursor_down(); // Should not go beyond last entry
        assert_eq!(state.cursor, 2);

        state.cursor_up();
        assert_eq!(state.cursor, 1);

        state.cursor_up();
        assert_eq!(state.cursor, 0);

        state.cursor_up(); // Should not go below 0
        assert_eq!(state.cursor, 0);
    }
}
