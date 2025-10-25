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
- [ ] Implementation in progress
- [ ] Testing & validation
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

### STEP 3: Infrastructure Implementations
**Time:** ~3.5 hours
**Status:** ⏳ PENDING

#### Tasks:
- [ ] Create `src/infrastructure/error.rs`:
  - [ ] `InfraError` enum with thiserror
  - [ ] Conversions from io::Error, tiberius::Error, etc.

- [ ] Refactor `src/repository.rs` → `src/infrastructure/filesystem_repository.rs`:
  - [ ] Implement `MigrationRepository` trait
  - [ ] Use domain types (ScriptPath, MigrationScript)
  - [ ] Keep existing logic but clean up API
  - [ ] Return domain errors instead of eyre::Result

- [ ] Refactor `src/script_memory.rs` → `src/infrastructure/sqlite_tracker.rs`:
  - [ ] Implement `ExecutionTracker` trait
  - [ ] Use domain types (Checksum, ScriptStatus)
  - [ ] Add proper error handling (no unwraps)
  - [ ] Keep database schema compatible (no breaking changes yet)

- [ ] Refactor `src/db.rs` → `src/infrastructure/mssql_executor.rs`:
  - [ ] Implement `ScriptExecutor` trait
  - [ ] Use domain types
  - [ ] Better error messages (context on failure)
  - [ ] Keep batch_parser.rs as-is (it's good)

- [ ] Update tests to use new infrastructure types
  - [ ] Update repository tests
  - [ ] Update CLI tests (merge logic)

**Risk:** Medium (changes interfaces, need to update call sites)
**Validation:** All existing tests still pass

---

### STEP 4: Service Layer
**Time:** ~2.5 hours
**Status:** ⏳ PENDING

#### Tasks:
- [ ] Create `src/services/action_dispatcher.rs`:
  - [ ] `ActionDispatcher` struct wrapping UnboundedSender
  - [ ] `dispatch()` method with error logging
  - [ ] `dispatch_async()` for spawning tasks
  - [ ] Centralized error handling

- [ ] Create `src/services/migration_service.rs`:
  - [ ] `MigrationService` struct
  - [ ] Dependencies: repository, executor, tracker (trait objects)
  - [ ] `execute_script()` method (extracted from scroll_list.rs)
  - [ ] `get_migration_status()` method (extracted from list.rs)
  - [ ] `calculate_checksums()` method
  - [ ] `validate_connection()` method (new!)
  - [ ] Proper error propagation with context

- [ ] Extract script execution logic from `scroll_list.rs` to service
  - [ ] Move CRC calculation to domain
  - [ ] Move database interaction to service
  - [ ] Keep UI component minimal

- [ ] Extract status calculation from `list.rs` to service
  - [ ] Move file reading to service
  - [ ] Move status determination to service

- [ ] Write service tests:
  - [ ] Mock repository/executor/tracker
  - [ ] Test execute_script success/failure paths
  - [ ] Test status calculation

**Risk:** Medium (changes component behavior)
**Validation:** Service tests pass, UI still works

---

### STEP 5: Refactor UI Components
**Time:** ~4.5 hours
**Status:** ⏳ PENDING

#### Tasks:
- [ ] Update `src/ui/component.rs`:
  - [ ] Add `with_dispatcher()` method to trait
  - [ ] Update trait documentation

- [ ] Refactor `src/ui/list.rs` → `src/ui/file_browser.rs`:
  - [ ] Remove business logic (move to service)
  - [ ] Use ActionDispatcher instead of raw channels
  - [ ] Simplify update() method to just UI state changes
  - [ ] Keep draw() mostly as-is (it's good)
  - [ ] Handle Action::StatusUpdated instead of calculating status

- [ ] Refactor `src/ui/scroll_list.rs` → `src/ui/execution_view.rs`:
  - [ ] Remove script execution logic (use service)
  - [ ] Use ActionDispatcher
  - [ ] Simplify to just display script execution results
  - [ ] Add visual progress indicator

- [ ] Refactor `src/ui/script_status.rs` → `src/ui/script_details.rs`:
  - [ ] Better error message display
  - [ ] Show execution time prominently
  - [ ] Show connection status

- [ ] Update `src/app.rs`:
  - [ ] Wire up services
  - [ ] Create ActionDispatcher
  - [ ] Inject dispatcher into components
  - [ ] Handle service errors gracefully
  - [ ] Simplify action handling (delegate to services)

- [ ] Update `src/main.rs`:
  - [ ] Build dependency graph (repository, executor, tracker)
  - [ ] Create services
  - [ ] Pass services to app
  - [ ] Add connection validation on startup

**Risk:** High (touches everything, lots of integration)
**Validation:** Manual testing of all UI flows

---

### STEP 6: User Experience Improvements
**Time:** ~3.5 hours
**Status:** ⏳ PENDING

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

**Currently Working On:** Step 1 - Foundation
**Last Updated:** 2025-10-25
**Next Session:** Continue with Step 1 tasks

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
