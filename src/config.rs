use std::{collections::HashMap, error::Error, fs::create_dir_all, path::PathBuf};

use color_eyre::eyre;
use config::{Config, Environment, File, FileFormat};
use directories::ProjectDirs;

pub fn read_config() -> eyre::Result<HashMap<String, String>, Box<dyn Error>> {
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

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "Eurowag", "aeq-cac")
}

pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

fn ensure_config_file() -> eyre::Result<PathBuf> {
    let config_dir = get_config_dir();

    create_dir_all(&config_dir)?;

    let config_path = config_dir.join("init.toml");

    Ok(config_path)
}
