use std::path::PathBuf;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Returned when Operating system fails a file operation.
    Io(std::io::Error),

    /// Data on disk is not what MortDB wrote (bad checksum,
    /// bad length, malformed record).
    Corruption {
        file: PathBuf,
        offset: u64,
        reason: String,
    },

    /// The caller passed an argument that the program's API
    /// cannot accept or process.
    InvalidArgument(String),

    /// The passed arguments(s) or request are valid but not
    /// supported by the current build or version.
    NotSupported(String),

    /// The config field is being set with an invalid value.
    InvalidConfig {
        field: Option<&'static str>,
        message: String,
    },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {err}"),
            Error::Corruption {
                file,
                offset,
                reason,
            } => write!(
                f,
                "Corruption in {} at offset {}: {}",
                file.display(),
                offset,
                reason
            ),
            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            Error::NotSupported(msg) => write!(f, "Not supported: {msg}"),
            Error::InvalidConfig {
                field: Some(field),
                message,
            } => {
                write!(f, "Invalid config field {field}: {message}")
            }
            Error::InvalidConfig {
                field: None,
                message,
            } => {
                write!(f, "Invalid config: {message}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn question_operator_propagates_invalid_argument_error() {
        let result = caller(-10);
        assert!(matches!(result, Err(Error::InvalidArgument(_))));
    }

    #[test]
    fn question_operator_passes_through_ok_value_for_valid_input() {
        let result = caller(100);
        assert!(matches!(result, Ok(10)));
    }

    #[test]
    fn question_operator_converts_io_error_to_ioerror_variant() {
        let result = read_missing_file();
        assert!(matches!(
            result,
            Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn display_works_for_corruption_error() {
        let corruption_err = Error::Corruption {
            file: PathBuf::from("00000000.sst"),
            offset: 4096,
            reason: "invalid character".to_string(),
        };

        let msg = corruption_err.to_string();
        assert!(msg.contains("00000000.sst"));
        assert!(msg.contains("4096"));
        assert!(msg.contains("invalid character"));
    }

    fn read_missing_file() -> Result<String> {
        let contents = std::fs::read_to_string("/path/to/nonexistent/file")?;
        Ok(contents)
    }

    fn caller(dividend: i32) -> Result<i32> {
        let div_by_10 = floor_div_by_10(dividend)?;
        Ok(div_by_10)
    }

    fn floor_div_by_10(dividend: i32) -> Result<i32> {
        if dividend < 0 {
            return Err(Error::InvalidArgument(
                "dividend cannot be a negative number.".to_string(),
            ));
        }
        Ok(dividend / 10)
    }
}
