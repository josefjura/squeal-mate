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

- **src/main.rs**: Entry point, handles CLI parsing and initialization
- **src/app.rs**: Application state and main run loop
- **src/cli.rs**: Command-line argument parsing using `clap`
- **src/config.rs**: Configuration loading from TOML files and environment variables (prefix: `SQUEALMATE_`)
- **src/db.rs**: SQL Server database connection and script execution using `tiberius`
- **src/repository.rs**: File system operations for browsing and reading SQL script files
- **src/batch_parser.rs**: Parses SQL scripts to split on `GO` statements (ignoring GO in strings/comments)
- **src/script_memory.rs**: SQLite-based local database tracking script execution history and CRC checksums
- **src/entries.rs**: Entry status tracking (Unknown, NeverStarted, Changed, Finished, etc.)
- **src/components/**: UI components
  - **list.rs**: File/directory browser component
  - **scroll_list.rs**: Script execution output display
  - **script_status.rs**: Script execution status panel
  - **help.rs**: Help overlay

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

## Important Notes

- This is a Rust project using Tokio for async runtime
- The application uses `tiberius` for SQL Server connectivity with support for integrated authentication (Kerberos/GSSAPI)
- The TUI is built with `ratatui` (modern fork of `tui-rs`)
- Files must have `.sql` extension to be recognized as migration scripts
- When adding new components, implement the `Component` trait and register them in the appropriate screen in `main.rs`
- Action handling follows the Elm architecture pattern - components emit actions that flow through the system
