use std::{collections::HashMap, error::Error, path::PathBuf};

use config::{Config, Environment, File, FileFormat};
use directories::ProjectDirs;

pub fn read_config() -> Result<HashMap<String, String>, Box<dyn Error>> {
    // Convert config path to string and handle potential error
    let config_dir = ensure_config_file()?;

    let config_path_str = config_dir
        .to_str()
        .ok_or("Configuration path is not valid UTF-8")?;

    let settings = Config::builder()
        .add_source(File::new(config_path_str, FileFormat::Toml).required(false))
        .add_source(Environment::with_prefix("AEQCAC"))
        .build()
        .map_err(|e| format!("Configuration build failed: {}", e))?;

    let config = settings
        .try_deserialize()
        .map_err(|e| format!("Configuration parsing failed: {}", e))?;

    Ok(config)
}

pub fn ensure_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let proj_dirs = ProjectDirs::from("com", "Eurowag", "aeq-cac")
        .ok_or("Cannot determine project directories")?;

    let config_dir = proj_dirs.config_dir();

    // Create the configuration directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(config_dir.to_path_buf())
}

fn ensure_config_file() -> Result<PathBuf, Box<dyn Error>> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join("init.toml");

    Ok(config_path)
}
