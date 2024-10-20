use color_eyre::eyre::{self};
use tracing::error;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::get_data_dir;

pub fn initialize_panic_handler() -> eyre::Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut t) = crate::tui::Tui::new() {
            if let Err(r) = t.exit() {
                error!("Unable to exit Terminal: {:?}", r);
            }
        }

        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            let meta = Metadata {
                version: env!("CARGO_PKG_VERSION").into(),
                name: env!("CARGO_PKG_NAME").into(),
                authors: env!("CARGO_PKG_AUTHORS").replace(':', ", ").into(),
                homepage: env!("CARGO_PKG_HOMEPAGE").into(),
            };

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        log::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

pub fn initialize_logging() -> eyre::Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(&directory)?;
    let log_path = directory.join("squealmate.log");
    let log_file = std::fs::File::create(log_path)?;

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}

// #[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
// pub enum PathWrapper {
//     Filename(String),
//     Relative {
//         relative_dir: PathBuf,
//         filename: String,
//     },
//     Absolute {
//         absolute_dir: PathBuf,
//         filename: String,
//     },
// }

// impl PathWrapper {
//     pub fn get_full_path(&self) -> Result<PathBuf, PathError> {
//         match self {
//             PathWrapper::Filename(_) => Err(PathError::CantCreateFromFilenameOnly),
//             PathWrapper::Relative {
//                 relative_dir,
//                 filename,
//             } => Ok(PathBuf::from(relative_dir).join(filename)),
//             PathWrapper::Absolute {
//                 absolute_dir,
//                 filename,
//             } => Ok(PathBuf::from(absolute_dir).join(filename)),
//         }
//     }
// }

// #[derive(Debug)]
// pub enum PathError {
//     CantCreateFromFilenameOnly,
//     CantReadDirectoryContents,
//     CantReadFile,
// }

// impl Display for PathError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Error while accessing a filename")
//     }
// }

// impl Error for PathError {}
