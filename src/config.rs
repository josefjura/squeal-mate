use std::{fs::create_dir_all, path::PathBuf};

use color_eyre::eyre;
use config::{Config, ConfigError, Environment, File, FileFormat};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(unused)]
pub struct Database {
    #[serde(default)]
    pub integrated: Option<bool>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(unused)]
pub struct Repository {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub repository: Repository,
}

#[derive(Debug)]
pub enum SettingError {
    NoConfigFile,
    NotAValidPath,
    InnerInitError(ConfigError),
    InnerDeserializationError(ConfigError),
}

impl Settings {
    pub fn new() -> Result<Self, SettingError> {
        let config_dir = ensure_config_file().map_err(|_| SettingError::NoConfigFile)?;

        let config_path_str = config_dir.to_str().ok_or(SettingError::NotAValidPath)?;

        Self::from_path(config_path_str)
    }

    pub fn from_path(config_path: &str) -> Result<Self, SettingError> {
        let s = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::new(config_path, FileFormat::Toml).required(false))
            .add_source(Environment::with_prefix("AEQCAC").separator("_"))
            .build()
            .map_err(SettingError::InnerInitError)?;

        s.try_deserialize()
            .map_err(SettingError::InnerDeserializationError)
    }

    pub fn default() -> Self {
        Self {
            database: Database {
                integrated: None,
                password: None,
                port: None,
                server: None,
                username: None,
                name: None,
            },
            repository: Repository { path: None },
        }
    }
}

// pub fn read_config() -> eyre::Result<HashMap<String, String>, Box<dyn Error>> {
//     // Convert config path to string and handle potential error
//     let config_dir = ensure_config_file()?;

//     let config_path_str = config_dir
//         .to_str()
//         .ok_or("Configuration path is not valid UTF-8")?;

//     let settings = Config::builder()
//         .add_source(File::new(config_path_str, FileFormat::Toml).required(false))
//         .add_source(Environment::with_prefix("AEQCAC"))
//         .build()
//         .map_err(|e| format!("Configuration build failed: {}", e))?;

//     let config = settings
//         .try_deserialize()
//         .map_err(|e| format!("Configuration parsing failed: {}", e))?;

//     Ok(config)
// }

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

pub fn get_logs_dir() -> PathBuf {
    let directory = if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
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

#[test]
fn empty_simple() {
    let s = Settings::from_path("./.tests/config/empty.toml");
    assert!(s.is_ok());
}

#[test]
fn path_only_simple() {
    let s = Settings::from_path("./.tests/config/path.toml");
    assert!(s.is_ok());
    assert_eq!(s.unwrap().repository.path, Some("PATH".to_string()))
}
