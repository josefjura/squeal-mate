# SquealMate Optimization & Cleanup Plan

**Date**: 2025-01-06
**Current State**: ~8,664 lines of code, 4 TODOs
**Goal**: Streamline codebase and improve performance

---

## 🎯 High Impact, Low Effort

### 1. SQLite Connection Pooling ⚡ **BIGGEST WIN**

**Current Problem**:
Every database query opens a new SQLite connection:
```rust
let conn = Connection::open(self.db_name.clone())?;
```

This happens:
- Every status check
- Every script record lookup
- Every skip/unskip operation
- During batch queries

**Impact on Performance**:
- Opening connections is expensive, especially on WSL→Windows FS
- With 3800+ scripts, this adds up fast
- Currently a major bottleneck

**Solution**:
- Use connection pooling (`r2d2` crate) OR
- Keep one persistent connection in `ScriptDatabase`
- Estimated improvement: **50-90% faster database operations**

**Files to Change**:
- `src/script_memory.rs` - Refactor to use connection pool

---

### 2. Operation-Scoped Filesystem Caching 🚀

**Current Problem**:
The 'n' (jump to next Not Run) feature scans all 3800 files every time:
```rust
file_explorer.list_sql_files_recursive(&root_dir).await
```
This takes 10-30 seconds on WSL→Windows FS.

**Better Solution** (operation-scoped, not time-based):
- Cache filesystem list **within a single operation**
- Example: 'S' (select to end) currently scans filesystem for EACH directory
- Cache the list once at the start of the operation, use for all directories
- Clear cache when operation completes
- Never risk showing stale data - user always sees current state

**When to Use**:
- `jump_to_next_not_run()` - Scan once, use the cached list
- `select_from_cursor_to_end()` - Scan once, process all directories from cache
- Multi-file operations that need file list multiple times

**When NOT to Use**:
- Between user actions (always fresh scan)
- After refresh/reload
- After file execution (files might have been added)

**Implementation**:
```rust
// In jump_to_next_not_run:
let cached_files = file_explorer.list_sql_files_recursive(&root_dir).await?;
// Use cached_files throughout this function
// Automatically dropped when function exits
```

**Estimated Improvement**: Operations that need file list stay the same speed, but operations that currently scan multiple times become much faster.

**Files to Change**:
- `src/ui/list.rs` - Pass cached file list through operations instead of re-scanning

---

### 3. Remove Unused Legacy Code 🧹

**Current Problem**:
The app was refactored to use Unified View as the only mode, but old code remains:

**Legacy Systems to Remove**:
1. **Old Screen Modes**:
   - `Mode::FileChooser` - No longer used
   - `Mode::ScriptRunner` - No longer used
   - Only `Mode::Unified` is active

2. **Old Components** (if not used in Unified View):
   - Check if `scroll_list.rs` is still needed
   - Check if `script_status.rs` is still needed
   - Old keybind handlers for removed modes

3. **Deprecated Methods**:
   - `leave_current_directory()`
   - `unselect_current()`
   - `select_all_after()`
   - `select_all_after_in_directory()`
   - Other methods marked as never used in warnings

4. **Old State Management**:
   - `NavigationState` enum (never used)
   - `ComponentState` struct (never constructed)

**Benefits**:
- Reduce codebase by ~15-20%
- Easier to understand and maintain
- Fewer compilation warnings
- Less cognitive overhead

**Files to Review**:
- `src/app.rs` - Remove old mode handling
- `src/screen.rs` - Simplify to single mode
- `src/ui/*.rs` - Remove unused components
- `src/ui/list.rs` - Remove deprecated methods

---

## 🔧 Medium Impact, Medium Effort

### 4. Consolidate Dual Status Systems

**Current Problem**:
Two parallel systems exist for the same purpose:

1. **Legacy System** (`script_memory.rs`):
   - Direct SQLite access
   - Simple, straightforward
   - Still used in many places

2. **Domain Layer** (`domain/tracker.rs`, `infrastructure/sqlite_tracker.rs`):
   - Abstract traits and implementations
   - Domain-driven design
   - Adds indirection

**Decision Needed**:
Pick one approach:

**Option A: Keep Domain Layer** (if planning to add more databases/trackers)
- Remove all direct `script_memory` usage
- Go full domain-driven design
- More abstraction, more flexible

**Option B: Remove Domain Layer** (simpler, practical)
- Domain layer adds overhead for a single-database TUI app
- Direct SQLite access is fine for this use case
- Less abstraction, easier to understand
- **Recommended for simplicity**

**Impact**: Reduce complexity, eliminate duplicate code paths

---

### 5. Create Status Conversion Helpers (DRY Principle)

**Current Problem**:
Status conversion logic is duplicated ~5+ times:

```rust
// Appears in multiple files:
match record.result {
    ScriptResult::Success => EntryStatus::Finished(true),
    ScriptResult::Error => EntryStatus::Finished(false),
    ScriptResult::Skipped => EntryStatus::Skipped,
}

// And:
match status {
    ScriptStatus::NeverRun => EntryStatus::NeverStarted,
    ScriptStatus::UpToDate => EntryStatus::Finished(true),
    ScriptStatus::Modified => EntryStatus::Changed,
    ScriptStatus::Failed { .. } => EntryStatus::Finished(false),
    ScriptStatus::Running => EntryStatus::Unknown,
    ScriptStatus::Skipped => EntryStatus::Skipped,
}
```

**Solution**:
Create helper functions:
```rust
impl From<ScriptResult> for EntryStatus { ... }
impl From<ScriptStatus> for EntryStatus { ... }
```

**Files with Duplicate Logic**:
- `src/services/migration_service.rs`
- `src/ui/list.rs` (multiple places)
- `src/infrastructure/sqlite_tracker.rs`

---

## 📊 Low Priority (Already Good)

### 6. Tree Flattening Optimization
- Already cached with `cache_dirty` flag
- Works well, not a bottleneck
- **No action needed**

### 7. Action Dispatcher Overhead
- Current async design works fine
- Optimization would be marginal
- **Not worth the refactoring effort**

### 8. Progress Indicators
- `StatusCalculationProgress` exists but not displayed
- Either remove it or show actual progress bar
- Low priority - works without it

---

## 🎯 Recommended Implementation Order

### Phase 1: Quick Wins (1-2 days)
1. ✅ Remove legacy modes and unused code
2. ✅ SQLite connection pooling
3. ✅ Status conversion helpers

### Phase 2: Bigger Refactors (3-5 days)
4. ✅ Filesystem caching for 'n' command
5. ✅ Consolidate status systems (decide on approach)

### Phase 3: Polish (1 day)
6. ✅ Remove remaining TODOs
7. ✅ Clean up compiler warnings
8. ✅ Update documentation

---

## 📈 Expected Results

**Performance**:
- Database operations: 50-90% faster
- Jump to next Not Run: 100-1000x faster (after first use)
- Overall app responsiveness: Significantly improved

**Maintainability**:
- ~1500-2000 fewer lines of code (-20%)
- Single clear architecture (not dual systems)
- Easier onboarding for new contributors

**Developer Experience**:
- Cleaner codebase
- Fewer warnings
- Clear separation of concerns

---

## 🚨 Risks & Considerations

1. **Connection Pooling**: Need to handle SQLite's single-writer limitation
2. **Filesystem Caching**: Must invalidate when files actually change
3. **Removing Domain Layer**: If Option B chosen, harder to add multiple database support later
4. **Breaking Changes**: Some refactoring may break existing tests

---

## 📝 Notes

- WSL→Windows FS is inherently slow - we're optimizing around this constraint
- Most optimizations focus on reducing I/O operations
- Maintain backward compatibility with existing databases
- Don't over-engineer - this is a TUI tool, not a web service
