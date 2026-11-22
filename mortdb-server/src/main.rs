mod logging;

use std::process::ExitCode;

use mortdb::Config;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Logging may not be up yet, so report startup failures directly.
            eprintln!("mortdb-server: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), BoxError> {
    let config = match std::env::args_os().nth(1) {
        Some(path) => Config::load(path)?,
        None => Config::default(),
    };

    logging::init_logging(config.log_level)?;

    config.validate_server()?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        data_dir = %config.data_dir.display(),
        config = ?config,
        "mortdb-server starting"
    );

    Ok(())
}
