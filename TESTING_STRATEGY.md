# Testing Strategy for SquealMate

## Current Test Coverage (49 tests)

### ✅ Well-Covered Areas

#### Domain Layer (10 tests)
- `domain::script` - Checksum validation, path validation (7 tests)
- `domain::script_status` - Status determination logic (4 tests)

#### Infrastructure Layer (8 tests)
- `infrastructure::config` - Configuration loading (2 tests)
- `infrastructure::file_explorer` - Directory listing, file reading (3 tests)
- `repository` - File system operations (6 tests)

#### UI State Management (7 tests)
- `ui::list_state` - Navigation state machine, cursor positioning (7 tests)

#### Parsers & CLI (10 tests)
- `batch_parser` - GO statement parsing (7 tests)
- `cli` - Argument parsing (3 tests)

#### Services (2 tests)
- `services::action_dispatcher` - Action dispatching (2 tests)

### ❌ Missing/Weak Coverage Areas

#### Critical Gaps

1. **Database Operations** (0 tests)
   - Script execution (`db.rs`)
   - SQLite tracking (`script_memory.rs`)
   - Connection handling
   - Transaction management
   - Error recovery

2. **Migration Service** (0 tests)
   - Script ordering
   - Execution orchestration
   - Status tracking
   - Error handling

3. **UI Components** (partial coverage)
   - List component (0 integration tests)
   - Script status component (0 tests)
   - Help component (0 tests)
   - Component interactions

4. **Application State** (0 tests)
   - Script selection management
   - Mode switching (FileChooser ↔ ScriptRunner)
   - Action handling in app.rs

5. **Error Scenarios** (minimal)
   - Network failures
   - File system errors
   - Invalid SQL
   - Database connection errors

6. **Edge Cases**
   - Empty directories
   - Large file sets
   - Long-running queries
   - Concurrent modifications

---

## Recommended Testing Strategy

### Level 1: Unit Tests (Fast, Isolated)

**Goal**: Test individual functions and small units in isolation with mocked dependencies.

#### Priority 1: Database Layer

```rust
// tests for src/db.rs
mod db_tests {
    #[tokio::test]
    async fn test_connect_success() { }

    #[tokio::test]
    async fn test_connect_invalid_credentials() { }

    #[tokio::test]
    async fn test_execute_simple_script() { }

    #[tokio::test]
    async fn test_execute_script_with_error() { }

    #[tokio::test]
    async fn test_execute_script_with_go_statements() { }

    #[tokio::test]
    async fn test_parse_sql_error_location() { }
}
```

#### Priority 2: Script Memory / Tracking

```rust
// tests for src/script_memory.rs
mod script_memory_tests {
    #[test]
    fn test_initialize_database() { }

    #[test]
    fn test_record_script_execution_success() { }

    #[test]
    fn test_record_script_execution_failure() { }

    #[test]
    fn test_get_script_status_never_run() { }

    #[test]
    fn test_get_script_status_modified() { }

    #[test]
    fn test_get_script_status_up_to_date() { }

    #[test]
    fn test_concurrent_access() { }
}
```

#### Priority 3: Migration Service

```rust
// tests for src/services/migration_service.rs
mod migration_service_tests {
    #[tokio::test]
    async fn test_execute_single_script() { }

    #[tokio::test]
    async fn test_execute_multiple_scripts_in_order() { }

    #[tokio::test]
    async fn test_stop_on_error() { }

    #[tokio::test]
    async fn test_status_updates_during_execution() { }
}
```

#### Priority 4: UI Components

```rust
// tests for src/ui/list.rs
mod list_component_tests {
    #[test]
    fn test_selection_toggle() { }

    #[test]
    fn test_select_all_after() { }

    #[test]
    fn test_directory_navigation_full_cycle() { }

    #[test]
    fn test_cursor_movement_boundaries() { }
}
```

### Level 2: Integration Tests (Medium Speed, Real Components)

**Goal**: Test multiple components working together with real implementations (but possibly test databases).

Create `tests/` directory at project root for integration tests.

#### Priority 1: End-to-End Script Execution

```rust
// tests/script_execution_integration.rs
#[tokio::test]
async fn test_full_script_execution_flow() {
    // Setup: Create temp directory with test SQL files
    // Setup: Create test SQLite database
    // Execute: Run scripts through MigrationService
    // Verify: Check script_memory has correct records
    // Verify: Check SQL Server has expected changes
}

#[tokio::test]
async fn test_script_execution_with_failure() {
    // Execute script that will fail
    // Verify error is captured correctly
    // Verify subsequent scripts are not run
    // Verify script_memory reflects failure
}

#[tokio::test]
async fn test_modified_script_detection() {
    // Execute script
    // Modify script content
    // Detect change via CRC
    // Verify status is "Changed"
}
```

#### Priority 2: File System Integration

```rust
// tests/filesystem_integration.rs
#[tokio::test]
async fn test_nested_directory_navigation() {
    // Create temp directory structure
    // Navigate through multiple levels
    // Verify correct entries at each level
}

#[tokio::test]
async fn test_hidden_files_excluded() {
    // Create directory with hidden files
    // Verify they don't appear in listings
}

#[tokio::test]
async fn test_large_directory_listing() {
    // Create directory with 1000+ files
    // Verify performance and correctness
}
```

#### Priority 3: State Management Integration

```rust
// tests/state_management_integration.rs
#[tokio::test]
async fn test_selection_persistence_across_navigation() {
    // Select files in directory
    // Navigate down to subdirectory
    // Navigate back up
    // Verify selections are preserved
}

#[tokio::test]
async fn test_action_flow_from_ui_to_service() {
    // Simulate user action (SelectCurrent)
    // Verify action flows through dispatcher
    // Verify state updates correctly
}
```

### Level 3: E2E Tests (Slowest, Full System)

**Goal**: Test the entire application including TUI, database, and file system.

**Note**: E2E tests for TUI apps are complex. Consider these alternatives:
- **Manual test scenarios** (documented checklist)
- **Headless testing** (if we can mock terminal I/O)
- **Contract tests** (verify key integration points)

#### Recommended Approach: Manual Test Scenarios

Instead of automated E2E, create a comprehensive manual test checklist:

```markdown
## Manual E2E Test Scenarios

### Scenario 1: First-Time Setup
1. Run `squealmate init`
2. Configure database connection
3. Launch `squealmate`
4. Verify UI loads correctly
5. Browse repository
6. Select and execute first script
7. Verify success in database

### Scenario 2: Incremental Updates
1. Launch with existing history
2. Navigate to subdirectory
3. Select multiple scripts
4. Execute batch
5. Verify all succeed
6. Check script_memory

### Scenario 3: Error Handling
1. Execute script with syntax error
2. Verify error message shows line number
3. Verify execution stops
4. Fix script
5. Re-execute
6. Verify success

### Scenario 4: WSL Environment
1. Run from WSL
2. Connect to Windows SQL Server
3. Verify connection works
4. Execute scripts
5. Verify no network issues

### Scenario 5: Navigation UX
1. Navigate into deeply nested directories
2. Navigate back up multiple levels
3. Verify cursor position is correct
4. Verify performance is acceptable

### Scenario 6: Help System
1. Press `?` in FileChooser mode
2. Verify correct shortcuts shown
3. Press Tab to switch to ScriptRunner
4. Press `?` again
5. Verify different shortcuts shown
```

---

## Implementation Plan

### Phase 1: Critical Database Tests (Week 1)
- [ ] Database connection tests (success/failure cases)
- [ ] Script execution tests (with mocked SQL Server)
- [ ] Script memory tests (SQLite operations)
- [ ] Error parsing tests

**Estimated**: 15-20 new tests

### Phase 2: Integration Tests (Week 2)
- [ ] Full script execution flow
- [ ] File system integration
- [ ] State management integration

**Estimated**: 10-15 new tests

### Phase 3: Edge Cases & Error Scenarios (Week 3)
- [ ] Network failures
- [ ] Large datasets
- [ ] Concurrent access
- [ ] Malformed SQL
- [ ] Permission errors

**Estimated**: 10-15 new tests

### Phase 4: Manual Testing & Documentation (Week 4)
- [ ] Create manual test checklist
- [ ] Document test data setup
- [ ] Run full manual regression
- [ ] Document known issues/limitations

---

## Testing Tools & Frameworks

### Current Setup
- `cargo test` - Built-in test framework
- `tokio::test` - Async test support
- `tempfile` - Temporary file/directory creation (in dev-dependencies)

### Recommended Additions

```toml
[dev-dependencies]
# Already have
tempfile = "3"

# Recommended additions
mockall = "0.12"           # Mocking framework for traits
wiremock = "0.6"           # HTTP mocking (if needed for future)
proptest = "1.0"           # Property-based testing
test-case = "3.0"          # Parameterized tests
serial_test = "3.0"        # Run tests serially when needed

# For database testing
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }  # If we switch to sqlx
```

### Test Helpers to Create

```rust
// tests/common/mod.rs
pub mod helpers {
    /// Create a temporary test repository with SQL files
    pub fn create_test_repository() -> TempDir { }

    /// Create a test SQLite database
    pub fn create_test_script_memory() -> ScriptDatabase { }

    /// Mock SQL Server connection (for testing without real DB)
    pub fn mock_sql_connection() -> MockConnection { }

    /// Create test script with specific content
    pub fn create_test_script(name: &str, content: &str) -> Script { }
}
```

---

## Success Criteria

### Quantitative
- ✅ 49 existing tests (baseline)
- 🎯 80+ total tests (Phase 1-3 complete)
- 🎯 Critical paths have integration tests
- 🎯 All database operations have unit tests
- 🎯 All error scenarios have tests

### Qualitative
- ✅ Confidence in refactoring
- ✅ Faster bug detection
- ✅ Documentation through tests
- ✅ Reproducible test cases for issues
- ✅ New features include tests

---

## Current Status

**Total Tests**: 49
**Coverage**: ~40% (estimated, focusing on domain/parsing)
**Gaps**: Database, migration service, UI integration
**Priority**: Database layer tests (highest risk area)
