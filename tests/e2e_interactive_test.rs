//! Interactive E2E tests with snapshots
//!
//! These tests simulate real user interactions (keypresses) and capture
//! snapshots at each step to verify both behavior and appearance.

mod support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use squealmate::{
    action::Action,
    app::AppState,
    script_memory::{ScriptDatabase, ScriptResult},
    ui::{list::List, Component},
};
use std::path::PathBuf;
use std::sync::Arc;
use support::MockFileSystem;
use tokio::sync::mpsc;

/// Helper for running interactive E2E tests
struct InteractiveTest {
    list: List,
    terminal: Terminal<TestBackend>,
    app_state: AppState,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl InteractiveTest {
    /// Create a new test harness with the given filesystem
    async fn new(root: PathBuf, mock_fs: MockFileSystem, db: ScriptDatabase) -> Self {
        let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

        let backend = TestBackend::new(80, 24);
        let terminal = Terminal::new(backend).unwrap();

        let (action_tx, action_rx) = mpsc::unbounded_channel();
        list.register_action_handler(action_tx.clone()).unwrap();

        let terminal_size = ratatui::layout::Size {
            width: 80,
            height: 24,
        };
        list.init(terminal_size).unwrap();

        let mut test = Self {
            list,
            terminal,
            app_state: AppState::new(),
            action_tx,
            action_rx,
        };

        // Wait for initial load
        test.wait_for_load().await;

        test
    }

    /// Wait for entries to load and statuses to be calculated
    async fn wait_for_load(&mut self) {
        let mut timeout = 50;
        let mut entries_loaded = false;
        let mut statuses_calculated = false;

        while timeout > 0 && (!entries_loaded || !statuses_calculated) {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            while let Ok(action) = self.action_rx.try_recv() {
                let _ = self.list.update(&mut self.app_state, action.clone());

                match action {
                    Action::EntriesLoaded(_) => entries_loaded = true,
                    Action::CalculateEntryStatus => {}
                    Action::EntryStatusChanged(_, _) => statuses_calculated = true,
                    _ => {}
                }
            }

            timeout -= 1;
        }
    }

    /// Simulate a key press and process resulting actions
    async fn press_key(&mut self, key: KeyCode) {
        let key_event = KeyEvent::new(key, KeyModifiers::empty());

        // Map key to action (simplified - in real app this is in event handler)
        let action = match key {
            KeyCode::Char('j') | KeyCode::Down => Some(Action::CursorDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::CursorUp),
            KeyCode::Char(' ') => Some(Action::SelectCurrent),
            KeyCode::Enter => Some(Action::DirectoryOpenSelected),
            KeyCode::Char('n') => Some(Action::JumpToNextNotRun),
            KeyCode::Char('s') => Some(Action::ToggleSkip),
            KeyCode::Right => Some(Action::DirectoryExpand),
            KeyCode::Left => Some(Action::DirectoryCollapse),
            KeyCode::Char('g') => Some(Action::CursorToTop),
            KeyCode::Char('G') => Some(Action::CursorToBottom),
            _ => None,
        };

        if let Some(action) = action {
            let _ = self.list.update(&mut self.app_state, action);

            // Process any resulting async actions
            self.process_actions().await;
        }
    }

    /// Process all pending actions (for async operations)
    /// Waits until no new actions arrive for a short period
    async fn process_actions(&mut self) {
        let mut idle_iterations = 0;
        let max_idle = 5; // Wait for 5 iterations with no actions
        let mut total_timeout = 100;

        while total_timeout > 0 && idle_iterations < max_idle {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let mut had_actions = false;
            while let Ok(action) = self.action_rx.try_recv() {
                had_actions = true;
                let _ = self.list.update(&mut self.app_state, action);
            }

            if had_actions {
                idle_iterations = 0; // Reset idle counter when we get actions
            } else {
                idle_iterations += 1;
            }

            total_timeout -= 1;
        }
    }

    /// Render and capture the current state as text lines
    fn snapshot(&mut self) -> Vec<String> {
        self.terminal
            .draw(|f| {
                self.list.draw(f, f.area(), &self.app_state).unwrap();
            })
            .unwrap();

        let buffer = self.terminal.backend().buffer();
        extract_lines(buffer)
    }
}

fn extract_lines(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    let height = buffer.area().height as usize;
    let width = buffer.area().width as usize;
    let mut lines = Vec::new();

    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = buffer.cell((x as u16, y as u16)).unwrap();
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }

    lines
}

// ============================================================================
// STATIC BASELINE TESTS
// ============================================================================

#[tokio::test]
async fn test_static_single_file() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql"]);
    let db = ScriptDatabase::new_test().unwrap();

    let mut test = InteractiveTest::new(root, mock_fs, db).await;
    let snapshot = test.snapshot();

    insta::assert_debug_snapshot!("static_single_file", snapshot);
}

#[tokio::test]
async fn test_static_multiple_files_mixed_status() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();

    // Create mixed statuses
    db.insert("001_init.sql".to_string(), 12345, ScriptResult::Success)
        .unwrap();
    db.insert("002_users.sql".to_string(), 67890, ScriptResult::Error)
        .unwrap();
    // 003 is never run

    let mut test = InteractiveTest::new(root, mock_fs, db).await;
    let snapshot = test.snapshot();

    insta::assert_debug_snapshot!("static_mixed_status", snapshot);
}

// ============================================================================
// INTERACTIVE NAVIGATION TESTS
// ============================================================================

#[tokio::test]
async fn test_navigate_down_with_j_key() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);
    let db = ScriptDatabase::new_test().unwrap();

    let mut test = InteractiveTest::new(root, mock_fs, db).await;

    // Initial state - cursor on first file
    let snapshot1 = test.snapshot();
    insta::assert_debug_snapshot!("nav_initial", snapshot1);

    // Press 'j' to move down
    test.press_key(KeyCode::Char('j')).await;
    let snapshot2 = test.snapshot();
    insta::assert_debug_snapshot!("nav_after_j", snapshot2);

    // Press 'j' again
    test.press_key(KeyCode::Char('j')).await;
    let snapshot3 = test.snapshot();
    insta::assert_debug_snapshot!("nav_after_jj", snapshot3);
}

#[tokio::test]
async fn test_navigate_up_with_k_key() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);
    let db = ScriptDatabase::new_test().unwrap();

    let mut test = InteractiveTest::new(root, mock_fs, db).await;

    // Move down twice first
    test.press_key(KeyCode::Char('j')).await;
    test.press_key(KeyCode::Char('j')).await;

    let snapshot1 = test.snapshot();
    insta::assert_debug_snapshot!("nav_before_k", snapshot1);

    // Press 'k' to move up
    test.press_key(KeyCode::Char('k')).await;
    let snapshot2 = test.snapshot();
    insta::assert_debug_snapshot!("nav_after_k", snapshot2);
}

// ============================================================================
// INTERACTIVE SELECTION TESTS
// ============================================================================

#[tokio::test]
async fn test_select_file_with_space() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql", "002_users.sql"]);
    let db = ScriptDatabase::new_test().unwrap();

    let mut test = InteractiveTest::new(root, mock_fs, db).await;

    // Initial - nothing selected, cursor on root folder
    assert_eq!(
        test.app_state.selected.len(),
        0,
        "Should have no selections initially"
    );

    // Move down to first file (cursor starts on root folder)
    test.press_key(KeyCode::Char('j')).await;

    // Press space to select first file
    test.press_key(KeyCode::Char(' ')).await;
    test.process_actions().await;

    // Verify selection in AppState (selection is shown via green color, not text)
    assert_eq!(
        test.app_state.selected.len(),
        1,
        "Should have 1 file selected"
    );
    assert_eq!(test.app_state.selected[0].relative_path, "001_init.sql");

    let snapshot1 = test.snapshot();
    insta::assert_debug_snapshot!("select_one_file", snapshot1);

    // Move down and select second file
    test.press_key(KeyCode::Char('j')).await;
    test.press_key(KeyCode::Char(' ')).await;
    test.process_actions().await;

    // Verify both files selected
    assert_eq!(
        test.app_state.selected.len(),
        2,
        "Should have 2 files selected"
    );
    assert!(test
        .app_state
        .selected
        .iter()
        .any(|s| s.relative_path == "001_init.sql"));
    assert!(test
        .app_state
        .selected
        .iter()
        .any(|s| s.relative_path == "002_users.sql"));

    let snapshot2 = test.snapshot();
    insta::assert_debug_snapshot!("select_two_files", snapshot2);
}

// ============================================================================
// INTERACTIVE DIRECTORY TESTS
// ============================================================================

#[tokio::test]
async fn test_expand_directory_with_enter() {
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "migrations/002_users.sql",
        "migrations/003_products.sql",
    ]);
    let db = ScriptDatabase::new_test().unwrap();

    let mut test = InteractiveTest::new(root, mock_fs, db).await;

    // Initial - migrations folder collapsed
    let snapshot1 = test.snapshot();
    insta::assert_debug_snapshot!("expand_initial", snapshot1);

    // Move to migrations folder (should be second item)
    test.press_key(KeyCode::Char('j')).await;

    // Press Enter to expand
    test.press_key(KeyCode::Enter).await;
    test.process_actions().await;

    let snapshot2 = test.snapshot();
    insta::assert_debug_snapshot!("expand_after_enter", snapshot2);
}
