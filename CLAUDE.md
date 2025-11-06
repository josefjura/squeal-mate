# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SquealMate is a TUI (Terminal User Interface) application for managing incremental SQL migration scripts for SQL Server databases. It allows developers to track, execute, and monitor SQL migration scripts with visual feedback about execution status.

## Build and Development Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release          # Release build
```

### Testing
```bash
cargo test                     # Run all tests
cargo test <test_name>         # Run a specific test
cargo test --package squealmate --lib -- <module>::<test>  # Run specific module test
```

### Running
```bash
cargo run                      # Run the application (launches migrations explorer)
cargo run -- init              # Initialize configuration
cargo run -- config            # Show configuration info
cargo run -- migrations        # Explicitly launch migrations explorer
cargo run -- --help            # Show help
```

### Database Connection Arguments
The application supports runtime database connection overrides:
```bash
cargo run -- --server <SERVER> --port <PORT> -n <DB_NAME> -u <USERNAME> -p <PASSWORD>
cargo run -- -i true           # Use integrated authentication (Windows Auth)
```

## Architecture

### Core Components

**TUI Architecture Pattern:**
The application follows a component-based TUI architecture with:
- **App** (src/app.rs): Main application loop that manages screens, state, and the action event system
- **Screens** (src/screen.rs): Two modes - `FileChooser` and `ScriptRunner`
- **Components** (src/components/): Reusable UI components implementing the `Component` trait
- **Actions** (src/action.rs): Event-driven communication between components via an action channel
- **TUI** (src/tui.rs): Terminal management, event loop, and rendering using `ratatui` and `crossterm`

**Key Architectural Patterns:**
1. **Action-based communication**: Components communicate through the `Action` enum sent via unbounded channels
2. **Component lifecycle**: `register_action_handler` → `register_config_handler` → `init` → `handle_events` / `update` / `draw`
3. **Shared state**: `AppState` contains the list of selected scripts with their execution status
4. **Dual-screen mode**: Users navigate between file selection and script execution screens

### Module Structure

**Clean Architecture with Domain-Driven Design:**

- **src/main.rs**: Entry point, handles CLI parsing and initialization
- **src/app.rs**: Application state and main run loop
- **src/cli.rs**: Command-line argument parsing using `clap`
- **src/screen.rs**: Screen modes (FileChooser, ScriptRunner)
- **src/action.rs**: Action enum for component communication
- **src/tui.rs**: Terminal management and event loop

**Domain Layer** (Business logic, zero external dependencies):
- **src/domain/script.rs**: ScriptPath, MigrationScript, Checksum value objects
- **src/domain/script_status.rs**: ScriptStatus enum and status calculation
- **src/domain/repository.rs**: MigrationRepository trait (abstraction)
- **src/domain/executor.rs**: ScriptExecutor trait (abstraction)
- **src/domain/tracker.rs**: ExecutionTracker trait (abstraction)
- **src/domain/error.rs**: Domain-specific error types

**Infrastructure Layer** (External systems):
- **src/infrastructure/config.rs**: Configuration loading from TOML/environment
- **src/infrastructure/file_explorer.rs**: Simple filesystem operations for UI
- **src/infrastructure/filesystem_repository.rs**: File-based MigrationRepository implementation
- **src/infrastructure/sqlite_tracker.rs**: SQLite-based execution tracking
- **src/infrastructure/mssql_executor.rs**: SQL Server script executor
- **src/infrastructure/error.rs**: Infrastructure error types

**Service Layer** (Application orchestration):
- **src/services/migration_service.rs**: High-level migration operations
- **src/services/action_dispatcher.rs**: Async action dispatching helper

**UI Layer** (Components):
- **src/ui/component.rs**: Component trait definition
- **src/ui/list.rs**: File/directory browser with ComponentState
- **src/ui/list_state.rs**: NavigationState machine and state reducers
- **src/ui/scroll_list.rs**: Script execution output display
- **src/ui/script_status.rs**: Script execution status panel with spinner
- **src/ui/help.rs**: Context-sensitive help overlay

**Legacy/Utilities**:
- **src/db.rs**: SQL Server database connection (wrapped by mssql_executor)
- **src/repository.rs**: File system operations (wrapped by filesystem_repository)
- **src/batch_parser.rs**: Parses SQL scripts to split on `GO` statements
- **src/script_memory.rs**: SQLite tracking (wrapped by sqlite_tracker)
- **src/entries.rs**: Entry status tracking (legacy, being phased out)

### Component System

All UI components implement the `Component` trait with these methods:
- `register_action_handler`: Receives action channel sender
- `register_config_handler`: Receives application configuration
- `init`: Initialize with terminal size
- `handle_events`: Process keyboard/mouse events
- `update`: React to actions and update state (only runs when screen is active)
- `draw`: Render the component (REQUIRED)

Components can emit actions to communicate with other components or trigger side effects.

### Configuration

Configuration is loaded from (in priority order):
1. Environment variables with `SQUEALMATE_` prefix (e.g., `SQUEALMATE_DATABASE_SERVER`)
2. Config file at platform-specific location:
   - Linux: `~/.config/squealmate/config.toml`
   - Windows: `C:\Users\<user>\AppData\Local\beardo\squealmate\config.toml`
   - macOS: `~/Library/Application Support/com.beardo.squealmate/config.toml`

Example config structure (see `config.toml.example`):
```toml
[repository]
path = "/path/to/migrations"

[database]
integrated = false
username = "user"
password = "password"  # Not recommended in production
server = "localhost"
port = 1433
name = "database_name"
encryption = "required"  # Options: "required", "optional", "not_supported"
trust_server_certificate = true  # For self-signed certificates (SQL Server 2022)
```

### SQL Script Tracking

- Scripts are tracked locally using SQLite (location: platform-specific data directory)
- CRC32 checksums detect if scripts have been modified since last execution
- Script statuses: Unknown, NeverStarted, Changed, Finished (success/error)
- Hidden files/directories (starting with `_` or `.`) are excluded from script listings

### Batch Parsing Logic

The `BatchParser` splits SQL scripts on `GO` statements while correctly handling:
- GO within string literals (ignored)
- GO within multi-line comments `/* ... */` (ignored)
- GO as part of other words like "GOals" (ignored)
- Only standalone GO with surrounding whitespace is treated as batch separator

### User Experience Features

**Progress Indicators:**
- Spinner during script execution (yellow "Working" indicator)
- "X/Y completed" counter in status bar
- "Calculating checksums: X/Y (Z%)" progress during CRC calculation
- Real-time script execution status updates

**Context-Sensitive Help:**
- Press `?` or `h` to toggle help overlay
- Different help content for FileChooser vs ScriptRunner modes
- Enhanced keyboard shortcuts with clear formatting
- "Getting Started" section guides new users

**Error Handling:**
- SQL errors show line numbers and code snippets
- Connection validation on startup with helpful error messages
- Config validation with fix suggestions
- Detailed error context for debugging

**Async & Responsive:**
- File loading runs in background (no UI blocking)
- CRC calculations run asynchronously
- Progress updates during long operations
- NavigationState machine prevents accessing data during loading

## Important Notes

- This is a Rust project using Tokio for async runtime
- The application uses `tiberius` for SQL Server connectivity with support for integrated authentication (Kerberos/GSSAPI)
- The TUI is built with `ratatui` 0.29.0 (modern fork of `tui-rs`)
- Files must have `.sql` extension to be recognized as migration scripts
- When adding new components, implement the `Component` trait and register them in the appropriate screen in `main.rs`
- Action handling follows the Elm architecture pattern - components emit actions that flow through the system
- **Architecture:** Clean separation of Domain → Infrastructure → Service → UI layers
- **State Management:** Reducer pattern in ComponentState for predictable state transitions
- **Testing:** 48 unit tests covering domain logic, infrastructure, services, and UI state
