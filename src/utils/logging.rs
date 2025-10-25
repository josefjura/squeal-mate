use color_eyre::eyre;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::infrastructure::config::get_data_dir;

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
