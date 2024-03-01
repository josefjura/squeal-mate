use std::collections::HashMap;

use config::{Config, File, FileFormat};
use directories::ProjectDirs;
// use serde::Deserialize;

// #[derive(Debug, Deserialize, Default)]
// pub struct AeqConfig {
//     path: String,
// }

pub fn setup_config() -> Result<HashMap<String,String>, String> {
    let proj_dirs = ProjectDirs::from("com", "Eurowag", "cac")
        .ok_or("Cannot determine project directories")?;
    let config_dir = proj_dirs.config_dir();
    let config_path = config_dir.join("init.toml");

    println!("{:?}", config_dir);

    // Create the configuration directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Convert config path to string and handle potential error
    let config_path_str = config_path.to_str()
        .ok_or("Configuration path is not valid UTF-8")?;

    let settings = Config::builder()
        .add_source(File::new(config_path_str, FileFormat::Toml).required(false))
        .build()
        .map_err(|e| format!("Configuration build failed: {}", e))?;

    settings.try_deserialize()
        .map_err(|e| format!("Configuration parsing failed: {}", e))
}
