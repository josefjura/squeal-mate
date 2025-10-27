mod action;
mod app;
mod batch_parser;
mod cli;
mod db;
mod domain;
mod entries;
mod error;
mod infrastructure;
mod repository;
mod screen;
mod script_memory;
mod services;
mod tui;
mod ui;
mod utils;

use crate::screen::{Mode, Screen};

use crate::app::App;
use crate::ui::list::List;
use clap::Parser;
use cli::{Command, SquealMateArgs};
use cliclack::{confirm, input, intro, note, outro};

use color_eyre::eyre;
use ui::help::Help;
use ui::script_status::ScriptStatus;
use ui::scroll_list::ScrollList;
use infrastructure::{get_config_dir, get_data_dir, Settings};
use crossterm::style::Stylize;
use crossterm::{execute, style::Print};
use db::Database;
use error::ArgumentsError;
use script_memory::ScriptDatabase;
use std::env;
use std::io::{self, stdout};
use std::path::Path;
use std::{io::Write, path::PathBuf, str::FromStr};
use utils::{initialize_logging, initialize_panic_handler};

async fn start_tui(config: Settings, connection: Database, force: bool) -> eyre::Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    eprintln!("🚀 Starting SquealMate...");

    let path: PathBuf = if let Some(ref content) = config.repository.path {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    eprintln!("📂 Repository: {}", path.display());

    let script_memory = ScriptDatabase::new().await?;
    eprintln!("💾 Script database initialized");

    // Validate that the path exists before proceeding
    if !path.exists() {
        log::error!("Repository path does not exist: {}", path.display());
        eprintln!("❌ ERROR: Repository path does not exist");
        eprintln!();
        eprintln!("Path: {}", path.display());
        eprintln!();
        eprintln!("To fix this:");
        eprintln!("  1. Run 'squealmate init' to set up a valid repository path");
        eprintln!("  2. Create the directory: mkdir -p {}", path.display());
        eprintln!("  3. Check your configuration: squealmate config");
        return Ok(());
    }

    {  // Start scope for infrastructure setup
            // Create infrastructure layer instances
            use std::sync::Arc;
            use infrastructure::{FilesystemRepository, MssqlExecutor, SqliteTracker};
            use services::MigrationService;

            eprintln!("🔧 Initializing infrastructure...");

            let fs_repo = Arc::new(
                FilesystemRepository::new(path.clone())
                    .map_err(|e| eyre::eyre!("Failed to create filesystem repository: {}", e))?
            );
            let executor = Arc::new(MssqlExecutor::new(connection.clone()));
            let tracker = Arc::new(
                SqliteTracker::new().await
                    .map_err(|e| eyre::eyre!("Failed to create tracker: {}", e))?
            );

            // Create service layer
            let migration_service = Arc::new(MigrationService::new(
                fs_repo.clone(),
                executor.clone(),
                tracker.clone(),
            ));

            // Test database connection before starting TUI (with timeout)
            eprintln!("🔌 Testing database connection...");
            log::info!("Testing database connection...");
            let test_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                migration_service.test_connection()
            ).await;

            match test_result {
                Ok(Ok(())) => {
                    eprintln!("✅ Database connection successful");
                    log::info!("Database connection successful");
                }
                Ok(Err(e)) => {
                    log::error!("Database connection failed: {}", e);

                    // Use detailed error formatting
                    let detailed_error = db::format_connection_error(&e.to_string(), &connection);
                    eprintln!("{}", detailed_error);

                    if force {
                        eprintln!();
                        eprintln!("⚠️  --force flag detected, starting anyway...");
                        eprintln!("   (You won't be able to execute scripts)");
                        eprintln!();
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    } else {
                        eprintln!();
                        eprintln!("Use --force (-f) to start anyway without database connection.");
                        return Err(eyre::eyre!("Database connection failed"));
                    }
                }
                Err(_) => {
                    log::error!("Database connection timed out after 5 seconds");
                    eprintln!("❌ Connection timed out after 5 seconds");
                    eprintln!();
                    eprintln!("Server: {}:{}", connection.server, connection.port);
                    eprintln!("Database: {}", connection.name);
                    eprintln!();
                    eprintln!("This usually means:");
                    eprintln!("  1. SQL Server is not running");
                    eprintln!("  2. Firewall is blocking the connection");
                    eprintln!("  3. Server address or port is incorrect");
                    eprintln!("  4. Network issue preventing connection");
                    eprintln!();
                    eprintln!("To diagnose:");
                    eprintln!("  - Verify SQL Server is running");
                    eprintln!("  - Test connectivity: ping {}", connection.server);
                    eprintln!("  - Test port: telnet {} {}", connection.server, connection.port);
                    eprintln!("  - Check firewall rules");
                    eprintln!();
                    eprintln!("To fix:");
                    eprintln!("  1. Check configuration: squealmate config");
                    eprintln!("  2. Reconfigure: squealmate init");

                    if force {
                        eprintln!();
                        eprintln!("⚠️  --force flag detected, starting anyway...");
                        eprintln!("   (You won't be able to execute scripts)");
                        eprintln!();
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    } else {
                        eprintln!();
                        eprintln!("Use --force (-f) to start anyway without database connection.");
                        return Err(eyre::eyre!("Database connection timed out"));
                    }
                }
            }

            eprintln!("🎨 Loading UI components...");

            // Create List component (uses simple FileExplorer for UI browsing)
            let mut list = List::new(path.clone(), script_memory.clone())?;
            list.set_migration_service(migration_service.clone());
            // Note: refresh_entries() will be called in init() after dispatcher is set up

            let script_status = ScriptStatus::new();

            let mut scroll_list = ScrollList::new(connection.clone(), path, script_memory);
            scroll_list.set_migration_service(migration_service.clone());

            let mut app = App::new(
                vec![
                    Screen::new(
                        Mode::FileChooser,
                        vec![Box::new(list), Box::new(Help::new())],
                    ),
                    Screen::new(
                        Mode::ScriptRunner,
                        vec![
                            Box::new(scroll_list),
                            Box::new(script_status),
                            Box::new(Help::new()),
                        ],
                    ),
                ],
                config,
            );

            app.run().await?;
            execute!(
                stdout(),
                Print("🦀 Thank you for using SquealMate 🦀\n".yellow())
            )?;
    }  // End infrastructure setup scope

    Ok(())
}

fn draw_config(stdout: &mut io::Stdout) -> eyre::Result<()> {
    let config_path = get_config_dir();
    let data_path = get_data_dir();
    let config_path_str = config_path.to_str().expect("Unknown host system").white();
    let data_path_str = data_path.to_str().expect("Unknown host system").white();
    let version = env!("CARGO_PKG_VERSION").white();
    let version_msg = format!("Version: {}\n", version);
    let config_msg = format!("Config src: {}\n", config_path_str);
    let data_msg = format!("Logs dir: {}\n", data_path_str);
    execute!(
        stdout,
        Print("🦀 SquealMate 🦀\n".yellow()),
        Print("\n"),
        Print(version_msg),
        Print("Edition: "),
        Print("Ultimate\n\n".white()),
        Print(config_msg),
        Print(data_msg)
    )?;

    stdout.flush()?;
    Ok(())
}

fn init_config() -> eyre::Result<()> {
    intro("SquealMate")?;

    let mut settings = Settings::default();

    let current = env::current_dir()?;
    let current_string = current.to_str().expect("Unknown host system").to_string();

    let repository: String = input("Where are the migrations stored?")
        .default_input(&current_string)
        .validate(|input: &String| {
            let exists = Path::new(input).exists();
            if exists {
                Ok(())
            } else {
                Err("Enter existing directory.")
            }
        })
        .interact()?;
    settings.repository.path = Some(repository);

    let database: String = input("Database url")
        .default_input("localhost")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Please enter a username")
            } else {
                Ok(())
            }
        })
        .interact()?;
    settings.database.server = Some(database);

    let port: String = input("Database port")
        .default_input("1433")
        .validate(|input: &String| match input.parse::<u16>() {
            Ok(_) => Ok(()),
            Err(_) => Err("Port must be a number"),
        })
        .interact()?;
    settings.database.port = Some(port.parse::<u16>().unwrap());

    let integrated: bool = confirm("Do you want to use integrated security to connect to database? (e.g. Windows Authentication)")
		.initial_value(true)
		.interact()?;
    settings.database.integrated = Some(integrated);

    if !integrated {
        let username: String = input("SQL user name")
            .validate(|input: &String| {
                if input.is_empty() {
                    Err("Username cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact()?;
        settings.database.username = Some(username);

        let store_password: bool = confirm(
            "Do you want to store the password in the configuration file? (Not recommended)",
        )
        .initial_value(false)
        .interact()?;

        if store_password {
            let password: String = input("SQL user password")
                .validate(|input: &String| {
                    if input.is_empty() {
                        Err("Password cannot be empty")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;
            settings.database.password = Some(password);
        }
    }

    let db_name: String = input("Datababase name")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Database name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()?;
    settings.database.name = Some(db_name);

    // Encryption settings (SQL Server 2022 requires encryption by default)
    cliclack::log::info("SQL Server 2022 and newer require encryption by default.")?;

    let trust_cert: bool = confirm(
        "Trust server certificate? (Required for self-signed certificates)"
    )
    .initial_value(true)
    .interact()?;
    settings.database.trust_server_certificate = Some(trust_cert);

    let encryption_prompt = cliclack::select("Encryption level:")
        .initial_value("required")
        .item("required", "Required (SQL Server 2022 default)", "Encryption must be used")
        .item("optional", "Optional", "Try encryption, fall back to unencrypted if needed")
        .item("not_supported", "Not supported", "For older SQL Server versions without encryption")
        .interact()?;
    settings.database.encryption = Some(encryption_prompt.to_string());

    if let Some(ref path) = settings.repository.path {
        cliclack::log::info(format!("Repository path: {}", path))?;
    }

    if let Some(ref integrated) = settings.database.integrated {
        if *integrated {
            cliclack::log::info("Using integrated authentication")?;
        } else {
            if let Some(ref username) = settings.database.username {
                cliclack::log::info(format!("SQl user name: {}", username))?;
            }
            if let Some(ref password) = settings.database.password {
                cliclack::log::info(format!("SQl user password: {}", password))?;
            }
        }
    }

    if let Some(ref server) = settings.database.server {
        cliclack::log::info(format!("Database server: {}", server))?;
    }
    if let Some(ref port) = settings.database.port {
        cliclack::log::info(format!("Database port: {}", port))?;
    }
    if let Some(ref db_name) = settings.database.name {
        cliclack::log::info(format!("Database name: {}", db_name))?;
    }
    if let Some(ref encryption) = settings.database.encryption {
        cliclack::log::info(format!("Encryption: {}", encryption))?;
    }
    if let Some(trust_cert) = settings.database.trust_server_certificate {
        cliclack::log::info(format!("Trust server certificate: {}", trust_cert))?;
    }

    let can_save: bool = confirm(
        "Do you want to save the configuration? (If you choose no, the configuration will be lost)",
    )
    .initial_value(true)
    .interact()?;

    if !can_save {
        outro("Configuration not saved!")?;
        return Ok(());
    }

    let result = settings.save();

    match result {
        Ok(_) => {
            outro("Configuration saved!")?;
        }
        Err(infrastructure::config::SettingSaveError::SerializationError(e)) => {
            log::error!("Serialization error: {}", e);
            outro("Error while saving configuration!")?;
        }
        Err(infrastructure::config::SettingSaveError::WriteError(e)) => {
            log::error!("Write error: {}", e);
            outro("Error while saving configuration!")?;
        }
    }

    stdout().flush()?;

    Ok(())
}

fn setup_database() -> eyre::Result<()> {
    use rand::Rng;

    intro("SquealMate Database Setup")?;

    note(
        "Setup Instructions",
        "This wizard will help you:\n\
         1. Generate a secure password for database user\n\
         2. Create a SQL script to set up the user\n\
         3. Save credentials to your config file"
    )?;

    // Load existing settings or create new
    let mut settings = Settings::new().unwrap_or_else(|_| Settings {
        repository: Default::default(),
        database: Default::default(),
    });

    // Get database name
    let db_name: String = input("Database name")
        .default_input(&settings.database.name.clone().unwrap_or_else(|| "master".to_string()))
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Database name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()?;

    // Get username
    let username: String = input("SQL user name for SquealMate")
        .default_input("squealmate_user")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Username cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()?;

    // Generate secure password
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                           abcdefghijklmnopqrstuvwxyz\
                           0123456789\
                           !@#$%^&*";
    let mut rng = rand::thread_rng();
    let password: String = (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();

    note("Generated Password", &password)?;

    // Generate SQL script
    let sql_script = format!(
        r#"-- SquealMate Database Setup Script
-- =====================================
-- Generated by: squealmate setup-db
-- Database: {}
-- Username: {}
--
-- IMPORTANT: Run this script as a user with sufficient permissions (sa or db_owner)

USE master;
GO

-- Create login if it doesn't exist
IF NOT EXISTS (SELECT name FROM sys.server_principals WHERE name = '{}')
BEGIN
    CREATE LOGIN [{}]
    WITH PASSWORD = '{}',
         DEFAULT_DATABASE = [{}],
         CHECK_POLICY = OFF,
         CHECK_EXPIRATION = OFF;
    PRINT 'Login [{}] created successfully.';
END
ELSE
BEGIN
    PRINT 'Login [{}] already exists.';
    -- Update password for existing login
    ALTER LOGIN [{}] WITH PASSWORD = '{}';
    PRINT 'Password updated for [{}].';
END
GO

-- Switch to target database
USE [{}];
GO

-- Create user if it doesn't exist
IF NOT EXISTS (SELECT name FROM sys.database_principals WHERE name = '{}')
BEGIN
    CREATE USER [{}] FOR LOGIN [{}];
    PRINT 'User [{}] created successfully.';
END
ELSE
BEGIN
    PRINT 'User [{}] already exists.';
END
GO

-- Grant permissions
-- Option: db_owner role (full permissions)
IF IS_ROLEMEMBER('db_owner', '{}') = 0
BEGIN
    ALTER ROLE db_owner ADD MEMBER [{}];
    PRINT 'Added [{}] to db_owner role.';
END
ELSE
BEGIN
    PRINT '[{}] is already a member of db_owner role.';
END
GO

PRINT '';
PRINT '========================================';
PRINT 'Setup complete!';
PRINT '========================================';
PRINT 'Login: {}';
PRINT 'Database: {}';
PRINT '';
PRINT 'Next step: Run squealmate and it will use these credentials.';
PRINT '========================================';
GO
"#,
        db_name, username,
        username, username, password, db_name,
        username, username, username, password, username,
        db_name,
        username, username, username, username, username,
        username, username, username, username,
        username, db_name
    );

    // Ask where to save the script
    let script_path = format!("setup-{}.sql", db_name);
    let save_location: String = input("Where should we save the SQL script?")
        .default_input(&script_path)
        .interact()?;

    // Write SQL script to file
    std::fs::write(&save_location, &sql_script)?;
    cliclack::log::success(format!("SQL script saved to: {}", save_location))?;

    // Ask if user wants to save to config
    let save_to_config = confirm("Save these credentials to your SquealMate config?")
        .initial_value(true)
        .interact()?;

    if save_to_config {
        settings.database.name = Some(db_name.clone());
        settings.database.username = Some(username);
        settings.database.password = Some(password);
        settings.database.integrated = Some(false);

        match settings.save() {
            Ok(_) => {
                cliclack::log::success("Credentials saved to config file!")?;
            }
            Err(e) => {
                cliclack::log::error(format!("Failed to save config: {:?}", e))?;
            }
        }
    }

    outro("Database setup complete!")?;

    note(
        "Next Steps",
        &format!(
            "1. Run the SQL script on your SQL Server:\n\
             →  sqlcmd -S localhost -i {}\n\
             \n\
             2. Or open {} in SQL Server Management Studio and execute it\n\
             \n\
             3. Run 'squealmate' to start using your migrations!",
            save_location, save_location
        )
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut stdout = io::stdout();

    let config = Settings::new().expect("Error while loading config!");

    let args = SquealMateArgs::parse();

    match args.command {
        Some(Command::Config) => {
            draw_config(&mut stdout)?;
        }
        Some(Command::Migrations) | None => {
            match args.connection.merge(&config) {
                Ok(conn) => start_tui(config, conn, args.force).await?,
                Err(ArgumentsError::MissingPassword) => {
                    eprintln!("❌ ERROR: Missing database password");
                    eprintln!();
                    eprintln!("To fix this, either:");
                    eprintln!("  1. Run 'squealmate init' to configure your database connection");
                    eprintln!("  2. Pass the password via: squealmate -p <PASSWORD>");
                    eprintln!("  3. Set SQUEALMATE_DATABASE_PASSWORD environment variable");
                    eprintln!("  4. Use integrated authentication: squealmate -i true");
                }
                Err(ArgumentsError::MissingUsername) => {
                    eprintln!("❌ ERROR: Missing database username");
                    eprintln!();
                    eprintln!("To fix this, either:");
                    eprintln!("  1. Run 'squealmate init' to configure your database connection");
                    eprintln!("  2. Pass the username via: squealmate -u <USERNAME>");
                    eprintln!("  3. Set SQUEALMATE_DATABASE_USERNAME environment variable");
                    eprintln!("  4. Use integrated authentication: squealmate -i true");
                }
                Err(ArgumentsError::MissingDBName) => {
                    eprintln!("❌ ERROR: Missing database name");
                    eprintln!();
                    eprintln!("To fix this, either:");
                    eprintln!("  1. Run 'squealmate init' to configure your database");
                    eprintln!("  2. Pass the database name via: squealmate -n <DATABASE>");
                    eprintln!("  3. Set SQUEALMATE_DATABASE_NAME environment variable");
                }
                Err(ArgumentsError::PortNotNumber) => {
                    eprintln!("❌ ERROR: Invalid port number");
                    eprintln!();
                    eprintln!("The port must be a number between 1 and 65535.");
                    eprintln!("Example: squealmate --port 1433");
                }
            };
        }
        Some(Command::Initialize) => init_config()?,
        Some(Command::SetupDatabase) => setup_database()?,
    }

    Ok(())
}
