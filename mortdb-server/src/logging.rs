/// Logging setup for the server process.

use std::io::IsTerminal;

use crate::BoxError;

use mortdb::LogLevel;
use tracing_subscriber::filter::LevelFilter;

/// Environment variable that overrides `log_level` from the config.
pub const LOG_LEVEL_ENV: &str = "MORTDB_LOG_LEVEL";

pub fn init_logging(config_level: LogLevel) -> Result<(), BoxError> {
    let env_value = std::env::var(LOG_LEVEL_ENV).ok();
    let level = resolve_level(config_level, env_value.as_deref())?;

    tracing_subscriber::fmt()
        .with_max_level(to_filter(level))
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .try_init()
}

fn resolve_level(config_level: LogLevel, env_value: Option<&str>) -> mortdb::Result<LogLevel> {
    match env_value {
        Some(value) => value.parse(),
        None => Ok(config_level),
    }
}

fn to_filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Trace => LevelFilter::TRACE,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Off => LevelFilter::OFF,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_level_is_used_when_env_is_not_set() {
        let level = resolve_level(LogLevel::Warn, None).unwrap();
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn env_value_overrides_config_level() {
        let level = resolve_level(LogLevel::Warn, Some("debug")).unwrap();
        assert_eq!(level, LogLevel::Debug);
    }

    #[test]
    fn invalid_env_value_is_an_error_and_not_a_fallback() {
        let result = resolve_level(LogLevel::Warn, Some("loud"));
        assert!(matches!(
            result,
            Err(mortdb::Error::InvalidConfig {
                field: Some("log_level"),
                ..
            })
        ));
    }

    #[test]
    fn every_level_maps_to_the_matching_filter() {
        assert_eq!(to_filter(LogLevel::Trace), LevelFilter::TRACE);
        assert_eq!(to_filter(LogLevel::Debug), LevelFilter::DEBUG);
        assert_eq!(to_filter(LogLevel::Info), LevelFilter::INFO);
        assert_eq!(to_filter(LogLevel::Warn), LevelFilter::WARN);
        assert_eq!(to_filter(LogLevel::Error), LevelFilter::ERROR);
        assert_eq!(to_filter(LogLevel::Off), LevelFilter::OFF);
    }
}
