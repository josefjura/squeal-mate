//! AppBuilder for creating testable App instances
//!
//! This builder allows full dependency injection for testing.

use color_eyre::eyre;
use squealmate::{
    action::Action,
    app::{App, AppState},
    infrastructure::{FileExplorer, FileSystem, Settings},
    screen::{Mode, Screen},
    script_memory::ScriptDatabase,
    ui::{
        command_bar::CommandBar, execution_log::ExecutionLog, help::Help, list::List,
        script_preview::ScriptPreview, script_status::ScriptStatus, unified_view::UnifiedView,
    },
};
use std::path::PathBuf;
use std::sync::Arc;

/// Builder for creating App instances with dependency injection for testing
pub struct AppBuilder {
    root_path: PathBuf,
    filesystem: Option<Arc<dyn FileSystem>>,
    database: Option<ScriptDatabase>,
    config: Settings,
}

impl AppBuilder {
    /// Create a new AppBuilder with default test configuration
    pub fn new_test() -> Self {
        Self {
            root_path: PathBuf::from("/test"),
            filesystem: None,
            database: None,
            config: Settings::default(),
        }
    }

    /// Set the root path for the repository
    pub fn with_root(mut self, path: PathBuf) -> Self {
        self.root_path = path;
        self
    }

    /// Inject a custom filesystem implementation (e.g., MockFileSystem)
    pub fn with_filesystem(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.filesystem = Some(fs);
        self
    }

    /// Inject a custom database (e.g., test database)
    pub fn with_database(mut self, db: ScriptDatabase) -> Self {
        self.database = Some(db);
        self
    }

    /// Set custom configuration
    pub fn with_config(mut self, config: Settings) -> Self {
        self.config = config;
        self
    }

    /// Build the App instance
    ///
    /// Note: This creates a minimal app suitable for testing with full dependency injection.
    pub fn build(self) -> eyre::Result<App> {
        // Use provided database or create a test one
        let script_memory = self
            .database
            .unwrap_or_else(|| ScriptDatabase::new_test().unwrap());

        // Use provided filesystem or create a real FileExplorer
        let filesystem = if let Some(fs) = self.filesystem {
            fs
        } else {
            Arc::new(FileExplorer::new(self.root_path.clone())?) as Arc<dyn FileSystem>
        };

        // Create List instances with injected filesystem
        let mut list = List::new_with_filesystem(
            self.root_path.clone(),
            script_memory.clone(),
            filesystem.clone(),
        )?;

        // Clone for unified view
        let mut list_for_unified = List::new_with_filesystem(
            self.root_path.clone(),
            script_memory.clone(),
            filesystem.clone(),
        )?;

        let script_status = ScriptStatus::new();
        let execution_log_for_runner = ExecutionLog::new();

        // Create unified view components
        let unified_view = UnifiedView::new(
            Box::new(list_for_unified),
            Box::new(ScriptPreview::new(self.root_path.clone())),
            Box::new(ExecutionLog::new()),
            Box::new(CommandBar::new()),
        );

        // Create app with screens
        let app = App::new(
            vec![
                Screen::new(Mode::Unified, vec![Box::new(unified_view)]),
                Screen::new(
                    Mode::FileChooser,
                    vec![Box::new(list), Box::new(Help::new())],
                ),
                Screen::new(
                    Mode::ScriptRunner,
                    vec![
                        Box::new(execution_log_for_runner),
                        Box::new(script_status),
                        Box::new(Help::new()),
                    ],
                ),
            ],
            self.config,
        );

        Ok(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_builder_creates_app() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("squealmate_test_app");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let app = AppBuilder::new_test()
            .with_root(temp_dir.clone())
            .build();

        match &app {
            Ok(_) => {},
            Err(e) => panic!("AppBuilder failed: {:?}", e),
        }

        let app = app.unwrap();
        assert_eq!(app.screens.len(), 3);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
