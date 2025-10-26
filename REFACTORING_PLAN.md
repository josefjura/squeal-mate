# SquealMate Refactoring Plan

**Started:** 2025-10-25
**Target:** Beta-ready release with clean architecture
**Estimated Effort:** ~20 hours
**Breaking Changes:** Accepted (will require config reinitialization)

---

## 🎯 Goals

1. **Perfect User Experience:** No weird errors, intuitive setup, responsive, informative
2. **Good Architecture:** Clean separation of concerns, testable, maintainable
3. **Fast Iteration:** Easy to add features after refactoring

---

## 📋 Progress Tracking

- [x] Assessment completed
- [x] Architecture design completed
- [x] Core implementation completed (Steps 1-5)
- [ ] UX improvements (Step 6)
- [ ] Testing & polish (Step 7)
- [ ] Beta release ready

---

## 🏗️ New Architecture

### Module Structure
```
src/
├── main.rs                      # Entry point (minimal)
├── cli.rs                       # CLI parsing (keep mostly as-is)
├── app.rs                       # App orchestration (simplified)
├── tui.rs                       # Terminal management (keep as-is)
│
├── domain/                      # NEW - Core business logic (zero external deps)
│   ├── mod.rs
│   ├── script.rs                # Script entity & value objects
│   ├── migration_repository.rs  # Abstract trait for script storage
│   ├── script_executor.rs       # Script execution orchestration
│   ├── script_status.rs         # Status calculation logic
│   └── error.rs                 # Domain-specific errors
│
├── infrastructure/              # NEW - External systems
│   ├── mod.rs
│   ├── filesystem_repository.rs # File system implementation
│   ├── sqlite_tracker.rs        # SQLite execution tracking
│   ├── mssql_executor.rs        # SQL Server execution
│   └── config.rs                # Configuration (moved from root)
│
├── services/                    # NEW - Application services
│   ├── mod.rs
│   ├── migration_service.rs     # High-level migration operations
│   └── action_dispatcher.rs     # Async action handling
│
├── ui/                          # RENAMED from components/
│   ├── mod.rs
│   ├── component.rs             # Component trait
│   ├── file_browser.rs          # Renamed from list.rs
│   ├── execution_view.rs        # Renamed from scroll_list.rs
│   ├── script_details.rs        # Renamed from script_status.rs
│   └── help.rs                  # Keep as-is
│
└── utils/                       # Utilities
    ├── mod.rs
    ├── logging.rs               # Moved from utils.rs
    └── panic.rs                 # Moved from utils.rs
```

---

## 📝 Implementation Steps

### ✅ STEP 0: Assessment (COMPLETED)
**Time:** 1 hour
**Status:** ✅ DONE

- [x] Analyzed codebase (~3,300 lines)
- [x] Identified critical issues
- [x] Documented problem areas
- [x] Created architecture design

**Key Findings:**
- 60% business logic in UI components
- Inconsistent error handling
- Manual async channel management everywhere
- Good component trait design (keep this)
- Good config loading (keep this)
- Excellent panic handling (keep this)

---

### ✅ STEP 1: Foundation (No Breaking Changes)
**Time:** ~30 minutes
**Status:** ✅ COMPLETED

#### Tasks:
- [x] Add dependencies to Cargo.toml:
  - [x] `thiserror = "2.0"`
  - [x] `anyhow = "1.0"`
  - [x] `async-trait = "0.1"`

- [x] Create new module structure (empty files):
  - [x] `src/domain/mod.rs`
  - [x] `src/infrastructure/mod.rs`
  - [x] `src/services/mod.rs`
  - [x] `src/ui/mod.rs`
  - [x] `src/utils/mod.rs`

- [x] Move existing files (no content changes yet):
  - [x] `src/config.rs` → `src/infrastructure/config.rs`
  - [x] `src/utils.rs` → split into `src/utils/logging.rs` and `src/utils/panic.rs`
  - [x] `src/components/` → `src/ui/` (just rename directory)

- [x] Update `src/main.rs` imports to use new paths
- [x] Update all module imports throughout codebase
- [x] Remove old `utils.rs` file

- [x] Verify compilation: `cargo build`
- [x] Verify tests still pass: `cargo test`

**Result:** ✅ All 23 tests passing, builds successfully with only unused import warnings

**Risk:** Low (just moving files around)
**Rollback:** Git revert if issues

---

### ✅ STEP 2: Domain Layer
**Time:** ~2.5 hours → Actual: ~45 minutes
**Status:** ✅ COMPLETED

#### Tasks:
- [x] Create `src/domain/error.rs`:
  - [x] `DomainError` enum with thiserror
  - [x] Error variants: ScriptNotFound, InvalidScript, ExecutionFailed, etc.

- [x] Create `src/domain/script.rs`:
  - [x] `ScriptPath` value object (newtype around PathBuf) with validation
  - [x] `Checksum` value object (newtype around u32)
  - [x] `MigrationScript` struct
  - [x] Helper methods (validate, has_changed_since, etc.)

- [x] Create `src/domain/script_status.rs`:
  - [x] `ScriptStatus` enum (NeverRun, UpToDate, Modified, Failed, Running)
  - [x] `ExecutionResult` struct
  - [x] Status determination logic from execution history

- [x] Create `src/domain/repository.rs`:
  - [x] `MigrationRepository` trait with async-trait
  - [x] Methods: list_scripts, read_script, get_children, etc.

- [x] Create `src/domain/executor.rs`:
  - [x] `ScriptExecutor` trait with execute and test_connection

- [x] Create `src/domain/tracker.rs`:
  - [x] `ExecutionTracker` trait
  - [x] Methods for recording and querying execution history

- [x] Write unit tests for domain types:
  - [x] Test ScriptPath validation (3 tests)
  - [x] Test Checksum calculation (2 tests)
  - [x] Test MigrationScript validation (3 tests)
  - [x] Test ScriptStatus transitions (4 tests)

**Result:** ✅ 12 new domain tests, all passing. Clean separation of concerns achieved.

**Risk:** Low (new code, doesn't touch existing)
**Validation:** All domain tests pass (12/12), all existing tests pass (23/23)

---

### ✅ STEP 3: Infrastructure Implementations
**Time:** ~3.5 hours → Actual: ~40 minutes
**Status:** ✅ COMPLETED

#### Tasks:
- [x] Create `src/infrastructure/error.rs`:
  - [x] `InfraError` enum with thiserror
  - [x] Conversions from io::Error, tiberius::Error, rusqlite::Error
  - [x] Conversion to DomainError

- [x] Create `src/infrastructure/filesystem_repository.rs`:
  - [x] Implement `MigrationRepository` trait
  - [x] Use domain types (ScriptPath, MigrationScript)
  - [x] Wrap existing Repository for backwards compatibility
  - [x] Store root_path to fix lifetime issues

- [x] Create `src/infrastructure/sqlite_tracker.rs`:
  - [x] Implement `ExecutionTracker` trait
  - [x] Use domain types (Checksum, ScriptStatus)
  - [x] Wrap existing ScriptDatabase
  - [x] Convert between EntryStatus and ScriptStatus

- [x] Create `src/infrastructure/mssql_executor.rs`:
  - [x] Implement `ScriptExecutor` trait
  - [x] Use domain types (MigrationScript, ExecutionResult)
  - [x] Wrap existing Database
  - [x] Handle BOM removal, timing, error handling

**Result:** ✅ All 35 tests passing. Infrastructure wraps existing implementations cleanly.

**Risk:** Low (wrapped existing code, no breaking changes)
**Validation:** All tests pass (23 original + 12 domain)

---

### ✅ STEP 4: Service Layer
**Time:** ~2.5 hours → Actual: ~35 minutes
**Status:** ✅ COMPLETED

#### Tasks:
- [x] Create `src/services/action_dispatcher.rs`:
  - [x] `ActionDispatcher` struct wrapping UnboundedSender
  - [x] `dispatch()` method with error logging
  - [x] `dispatch_async()` for spawning async tasks
  - [x] `dispatch_task()` for spawning general tasks
  - [x] `sender()` for backwards compatibility
  - [x] Added 2 unit tests

- [x] Create `src/services/migration_service.rs`:
  - [x] `MigrationService` struct with Arc<dyn> trait objects
  - [x] Dependencies: repository, executor, tracker
  - [x] `execute_script()` method - orchestrates execution with notifications
  - [x] `get_script_status()` method
  - [x] `calculate_statuses()` method - spawns async status calculation
  - [x] `test_connection()` method

**Result:** ✅ All 37 tests passing (23 original + 12 domain + 2 service). Clean service abstraction.

**Risk:** Low (new code, comprehensive tests)
**Validation:** All tests pass, builds without errors

---

### ⚠️ STEP 5: Refactor UI Components
**Time:** ~4.5 hours → Actual: ~2 hours
**Status:** ⚠️ PARTIALLY COMPLETE (Hybrid Implementation)

#### Tasks:
- [x] Update `src/main.rs`:
  - [x] Create infrastructure instances (FilesystemRepository, MssqlExecutor, SqliteTracker)
  - [x] Build dependency graph with Arc wrappers
  - [x] Create MigrationService
  - [x] Wire service into ScrollList via set_migration_service()

- [x] Refactor `src/ui/list.rs`:
  - [x] Add ActionDispatcher field
  - [x] Replace manual tokio::spawn with dispatcher.dispatch()
  - [x] Use dispatcher in open_selected_directory()
  - [x] Use dispatcher in leave_current_directory()
  - [x] Keep existing business logic for now (will move in Step 6)

- [x] Refactor `src/ui/scroll_list.rs`:
  - [x] Add ActionDispatcher and MigrationService fields
  - [x] Add set_migration_service() method
  - [x] Completely refactor Action::ScriptRun handler to use MigrationService.execute_script()
  - [x] Extract old implementation to execute_script_legacy() as fallback
  - [x] Use domain types (ScriptPath, MigrationScript)
  - [x] Service handles all notifications (ScriptRunning, ScriptFinished, ScriptError)

**Result:** ⚠️ HYBRID IMPLEMENTATION - Major operations migrated, file selection still uses old code

**What's Actually Complete:**
- ✅ ScrollList script execution: Fully migrated to MigrationService.execute_script()
- ✅ List status calculation: Fully migrated to MigrationService.calculate_statuses()
- ✅ ActionDispatcher: Used throughout for action dispatching
- ✅ Navigation: Uses FilesystemRepository (enter/leave/current directory)
- ⚠️ File selection: Still uses old Repository via .inner() (lines 206, 223, 243, 259, 269)
- ⚠️ Entry listing: Manual std::fs implementation in refresh_entries()

**What's Still Using Old Code:**
- `repository.inner().get_children()` - Not using MigrationRepository trait method
- `repository.inner().read_files_after()` - Not using MigrationRepository trait method
- `repository.inner().read_files_in_directory()` - Not using MigrationRepository trait method

**Evidence:** Compiler warnings show `list_scripts`, `get_children`, `get_scripts_after` as "never used"

**Reality Check:**
- We created the architecture but didn't fully migrate away from old Repository
- FilesystemRepository exists but only ~40% of trait methods are actually called
- List is hybrid: new architecture for major operations, old code via `.inner()` for file selection

**See ACTUAL_STATE.md for detailed analysis**

**Risk:** Low (tests pass, works, but not as clean as claimed)
**Validation:** All 37 tests pass, builds successfully, but architecture partially unused

---

### ✅ STEP 6: Fix Asynchronous Behavior
**Time:** ~4 hours → Actual: ~2.5 hours
**Status:** ✅ COMPLETED

#### Problem:
Currently mixing sync and async incorrectly:
- ✅ CRC calculations run async via `tokio::spawn()` - GOOD
- ❌ File operations use `block_on()` - BLOCKS UI THREAD
- ❌ 6 places in List component blocking on async operations
- ❌ No progress indicators during long operations
- ❌ UI freezes during file listing/selection

#### Root Cause:
The TUI components are synchronous (they implement `Component` trait with sync methods), but we need to call async repository methods. Currently using `block_on()` as a hack, which blocks the UI thread.

#### Solution Implemented:
1. **Separated navigation state from repository**
   - Moved `current_directory` to List component (UI concern)
   - Repository now stateless and thread-safe
   - Can be Arc-wrapped and shared with async tasks

2. **Made repository thread-safe**
   - Wrapped `FilesystemRepository` in `Arc<>`
   - Cheap cloning for async tasks
   - No mutable state in repository

3. **Converted file loading to async**
   - `refresh_entries()` now spawns async task
   - Shows loading state immediately
   - Sends `EntriesLoaded` action when complete
   - UI stays responsive

4. **Added progress indicators**
   - `EntryStatus::Loading` variant with hourglass emoji
   - `StatusCalculationProgress` action tracks CRC calculations
   - Bottom status bar shows "Calculating checksums: X/Y (Z%)"
   - Auto-hides when complete

5. **Documented remaining `block_on()` calls**
   - 4 calls in selection operations documented as acceptable
   - Fast operations (reading directory from memory)
   - Triggered by explicit user action
   - Converting would add complexity for minimal benefit

#### Tasks Completed:
- [x] Add `Loading` state to `EntryStatus`
- [x] Create background task for `refresh_entries()`
  - [x] Start with loading state
  - [x] Spawn async task to list files
  - [x] Send `EntriesLoaded(Vec<ListEntry>)` action when done
- [x] Add `EntriesLoaded` action handler in List component
- [x] Add progress tracking field to List struct
- [x] Add spinner/loading indicator to UI
- [x] Add progress for CRC calculations
  - [x] Track "X of Y files processed"
  - [x] Show current file being processed
  - [x] Display percentage complete
- [x] Document 4 remaining `block_on()` calls as acceptable

**Result:** ✅ Main file loading converted to async, CRC progress visible, UI responsive
**Remaining:** 4 documented `block_on()` calls in selection operations (acceptable)

**Risk:** Low (careful state management, all tests pass)
**Validation:** ✅ All 37 tests passing, builds without errors, async pattern established

---

### STEP 7: User Experience Improvements
**Time:** ~2 hours
**Status:** ⏳ PENDING (Partially done)

#### Tasks:
- [ ] Better error messages:
  - [ ] Connection errors: suggest checking server/credentials
  - [ ] File not found: show full path
  - [ ] Execution errors: show SQL error with line number if possible
  - [ ] Config errors: guide user to run `squealmate init`

- [ ] Add progress indicators:
  - [ ] Show spinner during script execution
  - [ ] Show "X of Y scripts completed"
  - [ ] Estimated time remaining (based on avg execution time)

- [ ] Connection validation:
  - [ ] Test database connection on startup
  - [ ] Clear error if connection fails before UI loads
  - [ ] Option to retry connection

- [ ] Improve help screen:
  - [ ] Add context-sensitive help (different help per screen)
  - [ ] Show keyboard shortcuts prominently
  - [ ] Add "Getting Started" section

- [ ] Config validation:
  - [ ] Validate repository path exists on load
  - [ ] Validate database config before connecting
  - [ ] Helpful error messages for invalid config

- [ ] Add config migration:
  - [ ] Detect old config format
  - [ ] Prompt user to migrate or reinitialize
  - [ ] Backup old config before migration

**Risk:** Low (mostly additive features)
**Validation:** Manual testing of all error scenarios

---

### STEP 7: Testing & Polish
**Time:** ~3.5 hours
**Status:** ⏳ PENDING

#### Tasks:
- [ ] Update dependencies:
  - [ ] `cargo update` to get latest compatible versions
  - [ ] Check for breaking changes in major deps
  - [ ] Test after update

- [ ] Add integration tests:
  - [ ] `tests/integration/migration_flow.rs` - end-to-end test
  - [ ] `tests/integration/error_handling.rs` - error scenarios
  - [ ] Mock database for integration tests

- [ ] Add property-based tests:
  - [ ] Use `proptest` or `quickcheck`
  - [ ] Test domain invariants
  - [ ] Test checksum calculation

- [ ] Fix all clippy warnings:
  - [ ] Run `cargo clippy --all-targets -- -D warnings`
  - [ ] Fix assert_eq! with bool
  - [ ] Fix map_or simplification
  - [ ] Fix any new warnings from refactoring

- [ ] Add rustdoc comments:
  - [ ] Document public API of domain types
  - [ ] Document services
  - [ ] Add examples in doc comments
  - [ ] Run `cargo doc --open` to review

- [ ] Update documentation:
  - [ ] Update README.md with new setup instructions
  - [ ] Update CLAUDE.md with new architecture
  - [ ] Add CONTRIBUTING.md with development guide
  - [ ] Add examples/ directory with sample configs

- [ ] Performance review:
  - [ ] Profile script execution
  - [ ] Check for unnecessary clones
  - [ ] Optimize hot paths if needed

**Risk:** Low (quality improvements)
**Validation:** All tests pass, clippy clean, docs render correctly

---

## 🎉 Beta Release Checklist

- [ ] All refactoring steps completed
- [ ] All tests passing (unit + integration)
- [ ] No clippy warnings
- [ ] Documentation complete
- [ ] Manual testing on Windows & Linux
- [ ] Performance acceptable (< 100ms UI response)
- [ ] Error messages are helpful
- [ ] README has clear setup instructions
- [ ] Create GitHub release with:
  - [ ] Changelog
  - [ ] Breaking changes notice
  - [ ] Migration guide for existing users
  - [ ] Binary releases for Windows/Linux

---

## 📊 Metrics to Track

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Lines of Code | ~3,300 | TBD | ~3,500-4,000 |
| Test Coverage | ~23 tests | TBD | 50+ tests |
| Clippy Warnings | 15+ | TBD | 0 |
| Business Logic in UI | ~60% | TBD | <10% |
| Avg. Script Execution Time | TBD | TBD | < 100ms overhead |

---

## 🔄 Current Status

**Currently Working On:** Steps 1-5 COMPLETED! Ready for Step 6 (UX improvements)
**Last Updated:** 2025-10-25
**Next Session:** Begin Step 6 - User Experience Improvements

### Progress Summary:
- ✅ **Step 1** (Foundation) - 30 min - Module structure created
- ✅ **Step 2** (Domain Layer) - 45 min - 12 domain tests added
- ✅ **Step 3** (Infrastructure) - 40 min - Wrapped existing code with traits
- ✅ **Step 4** (Service Layer) - 35 min - ActionDispatcher + MigrationService created
- ✅ **Step 5** (UI Refactor) - 1.5 hrs - Major cleanup of component business logic

**Total Time So Far:** ~3.5 hours (vs. estimated 13 hours for steps 1-5)
**Tests Passing:** 37/37 ✅
**Build Status:** ✅ Clean (only expected unused import warnings)

---

## 📝 Notes & Decisions

### Architecture Decisions:
- Using trait objects (dyn) for flexibility over generic constraints
- Keeping ratatui as-is (no TUI framework change)
- Batch parser stays unchanged (it's well-tested)
- Config file format will change (breaking change acceptable)

### Performance Considerations:
- CRC calculation on every directory change (acceptable for now)
- SQLite for execution tracking (good enough, no need for full DB)
- Async file I/O for script loading (keep responsive UI)

### Future Enhancements (Post-Beta):
- [ ] Rollback functionality
- [ ] Dry-run mode
- [ ] Multiple database support
- [ ] Script templates
- [ ] Export execution history
- [ ] Web UI option

---

## 🆘 Troubleshooting

### If compilation fails during refactoring:
1. Check import paths (common after moving files)
2. Verify all modules are declared in mod.rs
3. Check for missing trait bounds
4. Run `cargo clean` and rebuild

### If tests fail:
1. Check if domain types changed
2. Verify mock implementations match new traits
3. Check for changed error types
4. Look for removed/renamed methods

### If UI behaves strangely:
1. Check action dispatching (new pattern)
2. Verify service is wired up correctly in app.rs
3. Check for missing state updates
4. Review component lifecycle (init → update → draw)

---

## 📚 References

- Original assessment: See discussion on 2025-10-25
- Architecture inspiration: Elm Architecture, Hexagonal Architecture
- Rust patterns: https://rust-unofficial.github.io/patterns/
- Domain-Driven Design (lite): https://khalilstemmler.com/articles/domain-driven-design-intro/
