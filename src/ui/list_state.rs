//! State management for the List component
//!
//! This module consolidates all component state and provides reducer-style
//! state transition methods that are easy to test and reason about.

use std::path::PathBuf;
use crate::entries::{EntryStatus, ListEntry};

/// Navigation state machine - makes async flow explicit
#[derive(Debug, Clone)]
pub enum NavigationState {
    /// Actively browsing entries
    Browsing {
        path: PathBuf,
        entries: Vec<ListEntry>,
        cursor: usize,
    },
    /// Loading entries from filesystem
    Loading {
        path: PathBuf,
        /// Name to position cursor on after loading
        position_cursor_on: Option<String>,
    },
    /// Error occurred during loading
    Error {
        path: PathBuf,
        error: String,
    },
}

/// Consolidated state for the List component
/// All state updates happen through reducer methods
#[derive(Debug, Clone)]
pub struct ComponentState {
    /// Navigation state machine
    pub nav_state: NavigationState,

    /// CRC calculation progress (current, total)
    pub crc_progress: Option<(usize, usize)>,
}

impl ComponentState {
    /// Convenience getters for common access patterns
    pub fn current_directory(&self) -> &PathBuf {
        match &self.nav_state {
            NavigationState::Browsing { path, .. } => path,
            NavigationState::Loading { path, .. } => path,
            NavigationState::Error { path, .. } => path,
        }
    }

    pub fn entries(&self) -> &[ListEntry] {
        match &self.nav_state {
            NavigationState::Browsing { entries, .. } => entries,
            _ => &[],
        }
    }

    pub fn cursor(&self) -> usize {
        match &self.nav_state {
            NavigationState::Browsing { cursor, .. } => *cursor,
            _ => 0,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(&self.nav_state, NavigationState::Loading { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(&self.nav_state, NavigationState::Error { .. })
    }
}

impl ComponentState {
    /// Create new state with initial directory
    pub fn new(initial_directory: PathBuf) -> Self {
        Self {
            nav_state: NavigationState::Browsing {
                path: initial_directory,
                entries: Vec::new(),
                cursor: 0,
            },
            crc_progress: None,
        }
    }

    // ===== Reducer Methods =====
    // Pure state transition functions

    /// Start loading entries - transition to Loading state
    pub fn start_loading(&mut self) {
        let path = self.current_directory().clone();
        self.nav_state = NavigationState::Loading {
            path,
            position_cursor_on: None,
        };
    }

    /// Entries have been loaded - transition to Browsing state
    pub fn on_entries_loaded(&mut self, entries: Vec<ListEntry>) {
        let path = self.current_directory().clone();

        // Get pending cursor name if we're in Loading state
        let cursor = if let NavigationState::Loading { position_cursor_on, .. } = &self.nav_state {
            if let Some(name) = position_cursor_on {
                // Find entry by name
                entries.iter().position(|e| &e.name == name).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        self.nav_state = NavigationState::Browsing {
            path,
            entries,
            cursor,
        };
    }

    /// Handle loading error - transition to Error state
    pub fn on_loading_error(&mut self, error: String) {
        let path = self.current_directory().clone();
        self.nav_state = NavigationState::Error {
            path,
            error,
        };
    }

    /// Navigate into a directory - transition to Loading state
    pub fn on_navigation_down(&mut self, new_directory: PathBuf) {
        self.nav_state = NavigationState::Loading {
            path: new_directory,
            position_cursor_on: None,
        };
    }

    /// Navigate up to parent directory - transition to Loading state with cursor positioning
    pub fn on_navigation_up(&mut self) -> Option<String> {
        let current_path = self.current_directory().clone();

        // Get current directory name to position cursor on
        let old_name = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Move to parent
        if let Some(parent) = current_path.parent() {
            self.nav_state = NavigationState::Loading {
                path: parent.to_path_buf(),
                position_cursor_on: old_name.clone(),
            };
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

    /// Update status for a specific entry (only in Browsing state)
    pub fn update_entry_status(&mut self, relative_path: &str, status: EntryStatus) {
        if let NavigationState::Browsing { entries, .. } = &mut self.nav_state {
            if let Some(entry) = entries.iter_mut().find(|e| e.relative_path == relative_path) {
                entry.status = status;
            }
        }
    }

    /// Move cursor up (only in Browsing state)
    pub fn cursor_up(&mut self) {
        if let NavigationState::Browsing { cursor, .. } = &mut self.nav_state {
            if *cursor > 0 {
                *cursor = cursor.saturating_sub(1);
            }
        }
    }

    /// Move cursor down (only in Browsing state)
    pub fn cursor_down(&mut self) {
        if let NavigationState::Browsing { cursor, entries, .. } = &mut self.nav_state {
            if *cursor < entries.len().saturating_sub(1) {
                *cursor = cursor.saturating_add(1);
            }
        }
    }

    /// Get the currently selected entry (only in Browsing state)
    pub fn selected_entry(&self) -> Option<&ListEntry> {
        if let NavigationState::Browsing { entries, cursor, .. } = &self.nav_state {
            entries.get(*cursor)
        } else {
            None
        }
    }

    /// Get mutable reference to currently selected entry (only in Browsing state)
    pub fn selected_entry_mut(&mut self) -> Option<&mut ListEntry> {
        if let NavigationState::Browsing { entries, cursor, .. } = &mut self.nav_state {
            entries.get_mut(*cursor)
        } else {
            None
        }
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
        assert_eq!(state.current_directory(), &PathBuf::from("/test"));
        assert_eq!(state.entries().len(), 0);
        assert_eq!(state.cursor(), 0);
        assert!(!state.is_loading());
        assert!(state.crc_progress.is_none());
        assert!(matches!(state.nav_state, NavigationState::Browsing { .. }));
    }

    #[test]
    fn test_start_loading() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        state.start_loading();

        assert!(state.is_loading());
        assert!(matches!(state.nav_state, NavigationState::Loading { .. }));
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

        assert!(!state.is_loading());
        assert_eq!(state.entries().len(), 2);
        assert_eq!(state.cursor(), 0); // Default to first entry
        assert!(matches!(state.nav_state, NavigationState::Browsing { .. }));
    }

    #[test]
    fn test_entries_loaded_with_pending_cursor() {
        let mut state = ComponentState::new(PathBuf::from("/test"));
        // Set up Loading state with cursor positioning
        state.nav_state = NavigationState::Loading {
            path: PathBuf::from("/test"),
            position_cursor_on: Some("file2.sql".to_string()),
        };

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

        assert_eq!(state.cursor(), 1); // Should position on file2.sql
        assert!(matches!(state.nav_state, NavigationState::Browsing { .. }));
    }

    #[test]
    fn test_navigation_up() {
        let mut state = ComponentState::new(PathBuf::from("/test/subdir"));

        let old_name = state.on_navigation_up();

        assert_eq!(old_name, Some("subdir".to_string()));
        assert_eq!(state.current_directory(), &PathBuf::from("/test"));
        assert!(state.is_loading());
        if let NavigationState::Loading { position_cursor_on, .. } = &state.nav_state {
            assert_eq!(position_cursor_on, &Some("subdir".to_string()));
        } else {
            panic!("Expected Loading state");
        }
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

        // Set up Browsing state with entries
        state.nav_state = NavigationState::Browsing {
            path: PathBuf::from("/test"),
            entries: vec![
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
            ],
            cursor: 0,
        };

        assert_eq!(state.cursor(), 0);

        state.cursor_down();
        assert_eq!(state.cursor(), 1);

        state.cursor_down();
        assert_eq!(state.cursor(), 2);

        state.cursor_down(); // Should not go beyond last entry
        assert_eq!(state.cursor(), 2);

        state.cursor_up();
        assert_eq!(state.cursor(), 1);

        state.cursor_up();
        assert_eq!(state.cursor(), 0);

        state.cursor_up(); // Should not go below 0
        assert_eq!(state.cursor(), 0);
    }
}
