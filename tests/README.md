# E2E Testing Infrastructure

This directory contains the end-to-end (E2E) testing infrastructure for SquealMate. The tests are designed to verify application behavior without requiring real filesystem or database I/O.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Test Support Infrastructure](#test-support-infrastructure)
- [Writing E2E Tests](#writing-e2e-tests)
- [Test Patterns](#test-patterns)
- [Snapshot Testing](#snapshot-testing)
- [Running Tests](#running-tests)
- [Troubleshooting](#troubleshooting)

## Overview

### Goals

The E2E testing infrastructure enables:
- **Full dependency injection**: Mock filesystem, database, and other external dependencies
- **Action-based testing**: Send actions directly to the app and verify state changes
- **Component isolation**: Test individual components with mocked dependencies
- **Fast execution**: No real I/O means tests run in milliseconds
- **Deterministic behavior**: No flaky tests due to filesystem or timing issues

### Key Principles

1. **Mock Everything External**: Filesystem, database, network calls
2. **Test Behavior, Not Implementation**: Focus on what the app does, not how
3. **Use Real Components**: Test actual component code, not test doubles
4. **Keep Tests Fast**: All tests should complete in <100ms

## Architecture

### Dependency Injection Flow

```
Test Code
   ↓
AppBuilder (tests/support/app_builder.rs)
   ↓
   ├─→ MockFileSystem (tests/support/mock_filesystem.rs)
   ├─→ ScriptDatabase::new_test() (creates temp DB)
   └─→ Settings (test config)
   ↓
App with fully mocked dependencies
```

### Component Initialization

```rust
// 1. Build app with mocked dependencies
let app = AppBuilder::new_test()
    .with_root(PathBuf::from("/test"))
    .with_filesystem(Arc::new(mock_fs))
    .with_database(test_db)
    .build()?;

// 2. Initialize action system
let (action_tx, action_rx) = mpsc::unbounded_channel();
app.initialize_components(action_tx.clone(), Size { width: 80, height: 24 })?;

// 3. Send actions and verify behavior
action_tx.send(Action::CursorDown)?;
// ... assert state changes
```

## Test Support Infrastructure

### Support Modules

Located in `tests/support/`:

#### `mod.rs`
- Exports `AppBuilder` and `MockFileSystem`
- Main entry point for test infrastructure

#### `app_builder.rs`
Builder pattern for creating testable `App` instances.

**API:**
```rust
AppBuilder::new_test()
    .with_root(PathBuf)              // Set repository root
    .with_filesystem(Arc<dyn FileSystem>) // Inject mock filesystem
    .with_database(ScriptDatabase)   // Inject test database
    .with_config(Settings)           // Override config
    .build() -> Result<App>          // Create the app
```

**Example:**
```rust
let app = AppBuilder::new_test()
    .with_root(PathBuf::from("/test"))
    .with_filesystem(Arc::new(mock_fs))
    .build()?;
```

#### `mock_filesystem.rs`
In-memory filesystem implementation for testing.

**API:**
```rust
MockFileSystem::new(root: PathBuf) -> Self
    .add_file(relative_path: &str)      // Add single file
    .add_directory(relative_path: &str) // Add directory
    .with_files(&[&str]) -> Self        // Builder: add multiple files
    .with_directories(&[&str]) -> Self  // Builder: add multiple dirs
```

**Example:**
```rust
let mock_fs = MockFileSystem::new(PathBuf::from("/test"))
    .with_files(&[
        "001_init.sql",
        "002_users.sql",
        "migrations/003_products.sql",
    ]);
```

**Features:**
- ✅ Supports nested directories
- ✅ Automatic parent directory creation
- ✅ Implements `FileSystem` trait
- ✅ All operations are in-memory (instant)

### Database Testing

**Creating Test Databases:**
```rust
let db = ScriptDatabase::new_test()?;
```

Each call creates a unique temporary database file, ensuring test isolation.

**Pre-populating Data:**
```rust
let db = ScriptDatabase::new_test()?;
db.insert("001_init.sql".to_string(), 12345, ScriptResult::Success)?;
db.insert("002_users.sql".to_string(), 67890, ScriptResult::Error)?;
```

## Writing E2E Tests

### Test Template

```rust
#[tokio::test]
async fn test_my_feature() {
    // 1. Setup: Create mocked dependencies
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone())
        .with_files(&["001_init.sql", "002_users.sql"]);

    let db = ScriptDatabase::new_test().unwrap();

    // 2. Build app
    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .with_database(db)
        .build()
        .expect("Should build app");

    // 3. Initialize (if testing actions)
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let terminal_size = Size { width: 80, height: 24 };
    app.initialize_components(action_tx.clone(), terminal_size)?;

    // Drain initialization actions
    while action_rx.try_recv().is_ok() {}

    // 4. Test behavior
    action_tx.send(Action::CursorDown)?;

    // 5. Assert results
    assert_eq!(app.state.selected.len(), 0);
}
```

### Test Categories

#### 1. Component Initialization Tests
Test that components can be created and initialized properly.

**File:** `app_test.rs`

**Example:**
```rust
#[test]
fn test_app_initializes_with_mock_filesystem() {
    let app = AppBuilder::new_test()
        .with_root(PathBuf::from("/test"))
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("App should build successfully");

    assert_eq!(app.screens.len(), 3);
}
```

#### 2. Navigation Tests
Test cursor movement, directory navigation, and selection.

**File:** `e2e_navigation_test.rs`

**Example:**
```rust
#[tokio::test]
async fn test_navigation_methods_dont_panic() {
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs))?;

    list.cursor_down(3);
    list.cursor_up();
    list.go_to_top();
    list.go_to_bottom(3);

    // Should not panic
}
```

#### 3. Action-Based Tests
Test the action dispatch system and state changes.

**File:** `e2e_with_actions_test.rs`

**Example:**
```rust
#[tokio::test]
async fn test_action_channel_communication() {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    app.initialize_components(action_tx.clone(), terminal_size)?;

    // Drain init actions
    while action_rx.try_recv().is_ok() {}

    action_tx.send(Action::CursorUp)?;
    action_tx.send(Action::CursorDown)?;

    let actions: Vec<_> = std::iter::from_fn(|| action_rx.try_recv().ok()).collect();
    assert_eq!(actions.len(), 2);
}
```

#### 4. State Manipulation Tests
Test direct AppState manipulation.

**File:** `e2e_with_actions_test.rs`

**Example:**
```rust
#[tokio::test]
async fn test_app_state_manipulation() {
    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()?;

    app.state.add("001_init.sql".to_string());
    assert_eq!(app.state.selected.len(), 1);

    app.state.toggle("001_init.sql".to_string());
    assert_eq!(app.state.selected.len(), 0);
}
```

#### 5. Snapshot Tests
Test UI rendering and capture visual output for regression testing.

**File:** `e2e_snapshot_test.rs`

**Example:**
```rust
#[tokio::test]
async fn test_list_renders_with_files() {
    let mock_fs = MockFileSystem::new(root.clone())
        .with_files(&["001_init.sql", "002_users.sql"]);

    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs))?;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Render component
    terminal.draw(|f| {
        list.draw(f, f.area(), &app_state)?;
    })?;

    // Extract and snapshot text
    let lines = extract_text_lines(terminal.backend().buffer());
    insta::assert_debug_snapshot!("list_with_files", lines);
}
```

See [Snapshot Testing](#snapshot-testing) section for detailed documentation.

## Test Patterns

### Pattern 1: Testing Component Safety

**When to use:** Verify operations don't panic even with no data loaded.

```rust
#[tokio::test]
async fn test_operations_are_safe() {
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs))?;

    // Should not panic even without loaded entries
    list.cursor_down(5);
    list.select_current(&mut state);
    list.toggle_skip();
}
```

### Pattern 2: Testing with Pre-populated Database

**When to use:** Test behavior with existing execution history.

```rust
#[tokio::test]
async fn test_with_execution_history() {
    let db = ScriptDatabase::new_test()?;
    db.insert("001_init.sql".to_string(), 12345, ScriptResult::Success)?;
    db.insert("002_users.sql".to_string(), 67890, ScriptResult::Error)?;

    let list = List::new_with_filesystem(root, db, Arc::new(mock_fs))?;
    // ... test status display
}
```

### Pattern 3: Testing Action Flow

**When to use:** Verify actions are dispatched and received correctly.

```rust
#[tokio::test]
async fn test_action_dispatch() {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    app.initialize_components(action_tx.clone(), terminal_size)?;

    // Important: Drain initialization actions!
    while action_rx.try_recv().is_ok() {}

    action_tx.send(Action::SelectCurrent)?;

    let action = action_rx.try_recv()?;
    assert!(matches!(action, Action::SelectCurrent));
}
```

### Pattern 4: Testing Multiple Files/Directories

**When to use:** Test hierarchical navigation.

```rust
#[tokio::test]
async fn test_nested_directories() {
    let mock_fs = MockFileSystem::new(root.clone())
        .with_files(&[
            "001_init.sql",
            "migrations/002_users.sql",
            "migrations/2024/003_orders.sql",
        ]);

    // Test directory expansion, navigation, etc.
}
```

## Snapshot Testing

### Overview

Snapshot testing captures the rendered UI output and compares it against previously approved snapshots. Any changes to the UI will cause tests to fail, allowing you to review and approve intentional changes while catching unintended regressions.

**File:** `e2e_snapshot_test.rs`

### How It Works

1. **Render to TestBackend**: Components render to an in-memory buffer
2. **Extract text**: Convert the buffer to readable text lines
3. **Snapshot**: Save or compare against stored snapshot
4. **Review**: Use `cargo insta` to review any changes

### Snapshot Test Template

```rust
#[tokio::test]
async fn test_component_renders_state() {
    // 1. Setup mocked dependencies
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone())
        .with_files(&["001_init.sql", "002_users.sql"]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // 2. Setup TestBackend with desired terminal size
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // 3. Initialize component
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    list.register_action_handler(action_tx.clone()).unwrap();

    let terminal_size = ratatui::layout::Size {
        width: 80,
        height: 24,
    };
    list.init(terminal_size).unwrap();

    // 4. Trigger any async operations
    list.refresh_entries().unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. Render to test buffer
    let app_state = AppState::new();
    terminal
        .draw(|f| {
            list.draw(f, f.area(), &app_state).unwrap();
        })
        .unwrap();

    // 6. Extract text lines from buffer
    let buffer = terminal.backend().buffer();
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

    // 7. Create snapshot
    insta::assert_debug_snapshot!("test_component_state", lines);
}
```

### Example Snapshot Output

Snapshots are stored in `tests/snapshots/` as YAML files:

```yaml
---
source: tests/e2e_snapshot_test.rs
expression: lines
---
[
    "📂  /test",
    "╔══════════════════════════════════════════════╗",
    "║>> ▶ ? test                                   ║",
    "║   ▶ ✓ 001_init.sql                           ║",
    "║   ▶ ✗ 002_users.sql                          ║",
    "║   ▶ ? 003_products.sql                       ║",
    "║                                              ║",
    "╚═══════════════════════════Press h for help══╝",
]
```

### Working with Snapshots

#### First-Time Setup

Install `cargo-insta` CLI tool:
```bash
cargo install cargo-insta
```

#### Creating New Snapshots

When you first run a snapshot test, it will fail (no snapshot exists yet):

```bash
# Run tests - will fail with "snapshot missing"
cargo test --test e2e_snapshot_test

# Review and accept new snapshots
cargo insta review

# Or auto-accept all new snapshots
cargo insta test --test e2e_snapshot_test --accept
```

#### Reviewing Changed Snapshots

When UI changes cause snapshots to differ:

```bash
# Run tests - will fail with diff
cargo test

# Interactive review of all changes
cargo insta review

# Show what changed
cargo insta test --test e2e_snapshot_test --review
```

The interactive reviewer shows:
- Old snapshot (red)
- New snapshot (green)
- Options: Accept, Reject, Skip

#### Accepting/Rejecting Changes

```bash
# Accept all changes (when changes are intentional)
cargo insta accept

# Reject all changes (when changes are bugs)
cargo insta reject

# Accept specific snapshot
cargo insta accept <snapshot_name>
```

### Existing Snapshot Tests

#### 1. `test_list_renders_empty_state`
- **Tests**: Empty filesystem rendering
- **Verifies**: Initial state, borders, help text
- **Snapshot**: Shows empty list with prompt

#### 2. `test_list_renders_with_files`
- **Tests**: List with 3 SQL files
- **Verifies**: File display, formatting
- **Snapshot**: Shows file list in tree view

#### 3. `test_list_renders_with_execution_history`
- **Tests**: Files with mixed execution status
- **Pre-populated**: Success, Error, Never Run
- **Verifies**: Status indicators (✓, ✗, ?)
- **Snapshot**: Shows colored status markers

#### 4. `test_list_renders_nested_directories`
- **Tests**: Hierarchical directory structure
- **Files**: root/, migrations/, migrations/2024/
- **Verifies**: Tree indentation, folder icons
- **Snapshot**: Shows nested directory structure

### Best Practices for Snapshot Testing

#### ✅ DO

- **Review snapshots carefully** before accepting
- **Keep snapshots small** - test specific components/states
- **Use descriptive names** - `test_list_empty_not_loaded_state`
- **Test different terminal sizes** - 80x24, 100x30, etc.
- **Test edge cases** - empty, full, errors, loading states
- **Commit snapshots** to git - they're part of the test suite
- **Update snapshots intentionally** when UI changes
- **Use consistent terminal sizes** for comparable snapshots

#### ❌ DON'T

- **Auto-accept without review** - could hide bugs
- **Snapshot entire app screens** - too brittle, hard to debug
- **Test dynamic content** - timestamps, random IDs will cause flakes
- **Ignore failing snapshots** - they're catching regressions!
- **Manually edit snapshots** - always regenerate through tests
- **Snapshot raw Cell data** - use text lines instead
- **Test with variable content** - use fixed test data

### Snapshot Testing Workflow

1. **Write test** with expected UI behavior
2. **Run test** - it will fail (no snapshot)
3. **Review output** - verify it looks correct
4. **Accept snapshot** with `cargo insta accept`
5. **Commit snapshot** to git
6. **CI runs tests** - ensures UI stays consistent
7. **On UI changes**:
   - Tests fail with diff
   - Review changes
   - Accept if intentional, reject if bug

### Debugging Failed Snapshots

When a snapshot test fails:

```bash
# See the diff
cargo insta test --test e2e_snapshot_test --review

# Check what changed
git diff tests/snapshots/
```

**Common causes:**
- Intentional UI improvements ✅ (accept)
- Styling changes ✅ (accept)
- Bug in rendering ❌ (reject and fix)
- Change in test data ❌ (fix test)
- Terminal size changed ❌ (use consistent size)

### Advanced: Custom Snapshot Filters

For dynamic content that changes between runs:

```rust
use insta::Settings;

let mut settings = Settings::clone_current();
settings.add_filter(r"\d{4}-\d{2}-\d{2}", "[DATE]");
settings.bind(|| {
    insta::assert_debug_snapshot!("name", output);
});
```

This replaces dates like "2024-01-15" with "[DATE]" for consistent snapshots.

### Snapshot File Structure

```
tests/
├── e2e_snapshot_test.rs          # Snapshot test code
├── snapshots/                     # Snapshot storage
│   ├── e2e_snapshot_test__list_empty_state.snap
│   ├── e2e_snapshot_test__list_with_files.snap
│   ├── e2e_snapshot_test__list_with_execution_history.snap
│   └── e2e_snapshot_test__list_nested_directories.snap
└── README.md                      # This file
```

### CI Integration

In CI pipelines:

```bash
# Run tests - should pass if snapshots are committed
cargo test

# If snapshots are out of date, CI will fail with diff
# Developers must update and commit new snapshots
```

**Never** use `--accept` in CI - it would hide regressions!

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test File
```bash
cargo test --test e2e_navigation_test
cargo test --test e2e_with_actions_test
cargo test --test app_test
```

### Run Specific Test
```bash
cargo test test_app_initializes_with_mock_filesystem
cargo test test_navigation_methods_dont_panic
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run in Parallel
```bash
cargo test -- --test-threads=4
```

## Troubleshooting

### Issue: Components Emit Extra Actions

**Symptom:** Receiving more actions than expected after initialization.

**Cause:** Components dispatch actions during initialization (e.g., `EntriesLoading`).

**Solution:** Drain initialization actions before testing:
```rust
app.initialize_components(action_tx.clone(), terminal_size)?;

// Drain init actions
while action_rx.try_recv().is_ok() {}

// Now send your test actions
action_tx.send(Action::CursorDown)?;
```

### Issue: Tests Fail Due to Async Operations

**Symptom:** State not updated as expected.

**Cause:** Components use `tokio::spawn` for async operations.

**Solution:**
- Current tests focus on **safety** (doesn't panic) not **behavior** (state changes)
- Full behavioral testing requires event loop integration (future work)
- For now, verify operations complete without errors

```rust
// Good: Test that operation is safe
list.select_from_cursor_to_end(&mut state);
// Don't assert state changes - async operation not complete

// Bad: Asserting async results
assert_eq!(state.selected.len(), 5); // May fail!
```

### Issue: Path Does Not Exist Error

**Symptom:** `Path does not exist: /test`

**Cause:** Using fake path with real `FileExplorer`.

**Solution:** Either use `MockFileSystem` or create real temp directory:
```rust
// Option 1: Mock filesystem
let mock_fs = MockFileSystem::new(PathBuf::from("/test"));
let app = AppBuilder::new_test()
    .with_filesystem(Arc::new(mock_fs))
    .build()?;

// Option 2: Real temp directory
let temp_dir = std::env::temp_dir().join("test_dir");
std::fs::create_dir_all(&temp_dir)?;
let app = AppBuilder::new_test()
    .with_root(temp_dir.clone())
    .build()?;
```

### Issue: Database Conflicts

**Symptom:** Tests interfere with each other.

**Cause:** Shared database file.

**Solution:** Always use `ScriptDatabase::new_test()` which creates unique temp files:
```rust
// Good: Isolated database
let db = ScriptDatabase::new_test()?;

// Bad: Shared database
let db = ScriptDatabase::new()?; // Don't use in tests!
```

## Best Practices

### ✅ DO

- Use `MockFileSystem` for all filesystem operations
- Use `ScriptDatabase::new_test()` for database operations
- Drain initialization actions before asserting
- Test that operations don't panic
- Use descriptive test names
- Clean up temp directories after tests
- Use `#[tokio::test]` for async tests

### ❌ DON'T

- Use real filesystem paths in tests
- Share database instances between tests
- Assert on async operation results without event loop
- Test implementation details
- Skip test cleanup
- Make tests depend on execution order
- Use `unwrap()` without clear error messages

## Future Enhancements

### Planned Features

1. ~~**TestBackend Integration**~~ ✅ **COMPLETED**
   - ~~Render to test buffer~~ ✅
   - ~~Snapshot testing with `insta`~~ ✅
   - ~~UI output verification~~ ✅

2. **Full Event Loop Testing**
   - Process actions with component updates
   - Test complete user workflows
   - Verify state changes after async operations

3. **Mock Database** (Optional)
   - In-memory database implementation
   - Faster test execution
   - More control over database behavior
   - **Note**: `ScriptDatabase::new_test()` already provides test isolation

4. **Test Utilities**
   - Common test fixtures
   - Helper functions for common patterns
   - Test data builders

## Contributing

When adding new E2E tests:

1. Choose the appropriate test file based on what you're testing
2. Follow the test template above
3. Use descriptive test names: `test_<feature>_<scenario>`
4. Add documentation comments explaining the test purpose
5. Ensure tests are fast (<100ms) and isolated
6. Update this README if introducing new patterns

## Examples

See existing test files for comprehensive examples:

- **`app_test.rs`**: App initialization and configuration
- **`e2e_navigation_test.rs`**: Navigation, selection, and UI operations
- **`e2e_with_actions_test.rs`**: Action system and state management
- **`e2e_snapshot_test.rs`**: UI rendering and snapshot testing
- **`mock_filesystem_test.rs`**: MockFileSystem API usage

---

**Test Count:** 97 tests across 6 test files
**Test Coverage:** Components, actions, state, navigation, UI rendering, integration
**Average Test Time:** <50ms per test
**Snapshot Tests:** 4 snapshots capturing UI output
