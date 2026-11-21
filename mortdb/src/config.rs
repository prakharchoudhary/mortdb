use serde::Deserialize;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Directory where MortDB keeps all of its files (WAL, SSTables, metadata).
    ///
    /// Default: './mortdb-data'
    pub data_dir: PathBuf,

    /// Create the database in `data-dir` if none exists there.
    ///
    /// When `false`, opening a missing databse is an error.
    ///
    /// Default: `true`.
    pub create_if_missing: bool,

    /// Refuse to open if a database already exists in `data_dir`.
    ///
    /// Useful when a caller expects to start from an empty store.
    ///
    /// Default: `false`.
    pub error_if_exists: bool,

    /// Max permitted size of a key, in bytes, that a write may carry.
    ///
    /// Default: 1 KiB. Must be greater than 0.
    pub max_key_size_bytes: usize,

    /// Max permitted size of a value, in bytes, that a write may carry.
    ///
    /// Default: 1 MiB.
    pub max_value_size_bytes: usize,

    /// Size, in bytes, the memtable should reach before it is flushed to an SSTable.
    ///
    /// Larger value means fewer, bigger flushes and more memory in use.
    /// They also means more WAL to replay, after a crash.
    ///
    /// Default: 4 MiB. Must be able to hold at-least once entry of:
    /// `max_key_size_bytes` + `max_value_size_bytes`
    pub memtable_size_bytes: usize,

    /// When the WAL is forced to dusk relative to acknowledging a write.
    ///
    /// To understand in detail, refer [`WalSyncMode`]
    ///
    /// Default: [`WalSyncMode::Always`]
    pub wal_sync_mode: WalSyncMode,

    /// Target size, in bytes, of one data block inside an SSTable.
    ///
    /// A block is the unit read from disk and checksummed.
    ///
    /// Default:  4 KiB.
    pub sstable_block_size_bytes: usize,

    /// Bit of bloom filter stored per key in each SSTable.
    ///
    /// More bits lower the chance of reading a file that doe snot contain the key,
    /// at the cost of memory and disk. 10 bits give roughly a 1% false-positive rate.
    /// `0` disables bloom filters.
    ///
    /// Default: 10.
    pub sstable_bloom_bits_per_key: usize,

    /// Verify the checksum of every block read from the disk.
    ///
    /// Detects on-disk corruption at a small CPU cost per read.
    ///
    /// Default: `true`.
    pub read_verify_checksums: bool,

    /// Address the RESP-Protocol server binds to.
    ///
    /// Default: `127.0.0.1`.
    pub server_listen_addr: String,

    /// TCP Port the RESP-Protocol server binds to.
    ///
    /// Default: 6767.
    pub server_listen_port: u16,

    /// Maximum number of client connections accepted at once.
    ///
    /// Default: 1024.
    pub server_max_connections: usize,

    /// How the server secures client connections. See [`ServerSecurityMode`].
    ///
    /// Default: [`ServerSecurityMode::Insecure`].
    pub server_security_mode: ServerSecurityMode,

    /// Path to the TLS certificate file (PEM).
    ///
    /// No default. Required when `server_security_mode` is `tls`; ignored otherwise.
    pub server_tls_cert_path: Option<PathBuf>,

    /// Path to the TLS private key file (PEM).
    ///
    /// No default. Required when `server_security_mode` is `tls`; ignored otherwise.
    pub server_tls_key_path: Option<PathBuf>,

    /// Shared secret that clients must present to authenticate.
    ///
    /// No default. Required when `server_security_mode` is `tls`; ignored otherwise.
    pub server_auth_secret: Option<String>,

    /// Number of similar sized SSTables that must accumulate before they are merged into
    /// one.
    ///
    /// A lower value compacts sooner, reads check fewer tables but data is rewritten more often.
    /// A higher value compacts less and leaves more tables for each reads to check from.
    ///
    /// Default: 4, must be at-least 2.
    pub compaction_min_tables_per_tier: usize,

    /// How much can an SSTable's size vary from its tier's average and still count as similar-sized.
    ///
    /// Default: 1.5. Must be greater than 1.0.
    pub compaction_size_ratio: f32,
}

/// When the write-ahead log is forced to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalSyncMode {
    /// Sync the WAL to disk before acknowledging each write.
    ///
    /// Slowest mode, but an acknowledged write survives all catastrophes,
    /// except an obliterated disk.
    Always,

    /// Never sync explicitly, the OS flushes when it chooses.
    ///
    /// Fast but no guarantees that an acknowledged write survives power loss.
    Never,
}

/// How the server secures client connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerSecurityMode {
    /// Connections use TLS and clients must present the shared secret.
    ///
    /// Requires `server_tls_cert_path`, `server_tls_key_path` and
    /// `server_auth_secret` to be set.
    Tls,

    /// Plain TCP with no encryption and no authentication.
    ///
    /// Intended for local development and testing only.
    Insecure,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            data_dir: PathBuf::from("./mortdb-data"),
            create_if_missing: true,
            error_if_exists: false,
            max_key_size_bytes: 1024,
            max_value_size_bytes: 1024 * 1024,
            memtable_size_bytes: 4 * 1024 * 1024,
            wal_sync_mode: WalSyncMode::Always,
            sstable_block_size_bytes: 4 * 1024,
            sstable_bloom_bits_per_key: 10,
            read_verify_checksums: true,
            server_listen_addr: "127.0.0.1".to_string(),
            server_listen_port: 6767,
            server_max_connections: 1024,
            server_security_mode: ServerSecurityMode::Insecure,
            server_tls_cert_path: None,
            server_tls_key_path: None,
            server_auth_secret: None,
            compaction_min_tables_per_tier: 4,
            compaction_size_ratio: 1.5,
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path_as_str = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&path_as_str).map_err(|e| Error::InvalidConfig {
            field: None,
            message: e.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.data_dir.as_os_str().is_empty() {
            return Err(invalid("data_dir", "cannot be empty"));
        }

        if !self.create_if_missing && self.error_if_exists {
            return Err(invalid(
                "create_if_missing",
                "cannot be false while error_if_exists is true, since no database could ever be opened",
            ));
        }

        if self.max_key_size_bytes == 0 {
            return Err(invalid("max_key_size_bytes", "must be greater than 0"));
        }

        if self.max_value_size_bytes == 0 {
            return Err(invalid("max_value_size_bytes", "must be greater than 0"));
        }

        let max_entry_bytes = self
            .max_key_size_bytes
            .checked_add(self.max_value_size_bytes) // Protection against addition overflowing.
            .ok_or_else(|| {
                invalid(
                    "max_value_size_bytes",
                    "max_key_size_bytes + max_value_size_bytes is too large",
                )
            })?;

        if self.memtable_size_bytes < max_entry_bytes {
            return Err(invalid(
                "memtable_size_bytes",
                "must be at least max_key_size_bytes + max_value_size_bytes",
            ));
        }

        if self.sstable_block_size_bytes == 0 {
            return Err(invalid(
                "sstable_block_size_bytes",
                "must be greater than 0",
            ));
        }

        if self.sstable_bloom_bits_per_key > 32 {
            return Err(invalid(
                "sstable_bloom_bits_per_key",
                "must be at most 32 (0 disables filters)",
            ));
        }

        if self.compaction_min_tables_per_tier < 2 {
            return Err(invalid(
                "compaction_min_tables_per_tier",
                "must be at least 2",
            ));
        }

        if !self.compaction_size_ratio.is_finite() || self.compaction_size_ratio <= 1.0 {
            return Err(invalid(
                "compaction_size_ratio",
                "must be a finite number greater than 1.0",
            ));
        }

        Ok(())
    }

    pub fn validate_server(&self) -> Result<()> {
        let _listen_addr: IpAddr = self
            .server_listen_addr
            .parse()
            .map_err(|_| invalid("server_listen_addr", "must be a valid IP address"))?;

        if self.server_max_connections == 0 {
            return Err(invalid("server_max_connections", "must be at least 1"));
        }

        match self.server_security_mode {
            ServerSecurityMode::Tls => {
                let required = "required when server_security_mode is \"tls\"";

                match &self.server_tls_cert_path {
                    Some(path) if !path.as_os_str().is_empty() => {}
                    _ => return Err(invalid("server_tls_cert_path", required)),
                }

                match &self.server_tls_key_path {
                    Some(path) if !path.as_os_str().is_empty() => {}
                    _ => return Err(invalid("server_tls_key_path", required)),
                }

                match &self.server_auth_secret {
                    Some(secret) if !secret.is_empty() => {}
                    _ => return Err(invalid("server_auth_secret", required)),
                }
            }
            ServerSecurityMode::Insecure => {
                if self.server_tls_cert_path.is_some()
                    || self.server_tls_key_path.is_some()
                    || self.server_auth_secret.is_some()
                {
                    log::warn!(
                        "server_security_mode is \"insecure\": TLS and auth settings are ignored"
                    );
                }
            }
        }

        Ok(())
    }
}

fn invalid(field: &'static str, reason: &str) -> Error {
    Error::InvalidConfig {
        field: Some(field),
        message: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn assert_invalid<T: std::fmt::Debug>(result: Result<T>, expected_field: &str) {
        match result {
            Err(Error::InvalidConfig {
                field: Some(field), ..
            }) => {
                assert_eq!(field, expected_field)
            }
            other => panic!("expected InvalidConfig for {expected_field}, got {other:?}"),
        }
    }

    fn tls_config() -> Config {
        Config {
            server_tls_cert_path: Some(PathBuf::from("cert.pem")),
            server_tls_key_path: Some(PathBuf::from("key.pem")),
            server_auth_secret: Some("secret".to_string()),
            ..Config::default()
        }
    }

    fn write_config_file(test_name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mortdb-config-test-{}-{}.toml",
            std::process::id(),
            test_name
        ));
        fs::write(&path, contents).expect("failed to write temporary config file");
        path
    }

    #[test]
    fn default_config_passes_validation() {
        let result = Config::default().validate();
        assert!(
            result.is_ok(),
            "default config should be valid, got {result:?}"
        );
    }

    #[test]
    fn default_config_passes_server_validation() {
        let result = Config::default().validate_server();
        assert!(
            result.is_ok(),
            "default config should be valid for the server, got {result:?}"
        );
    }

    #[test]
    fn default_storage_settings_match_documented_values() {
        let config = Config::default();
        assert_eq!(config.data_dir, PathBuf::from("./mortdb-data"));
        assert!(config.create_if_missing);
        assert!(!config.error_if_exists);
        assert_eq!(config.max_key_size_bytes, 1024);
        assert_eq!(config.max_value_size_bytes, 1024 * 1024);
        assert_eq!(config.memtable_size_bytes, 4 * 1024 * 1024);
        assert_eq!(config.sstable_block_size_bytes, 4 * 1024);
        assert_eq!(config.sstable_bloom_bits_per_key, 10);
        assert!(config.read_verify_checksums);
    }

    #[test]
    fn default_server_settings_match_documented_values() {
        let config = Config::default();
        assert_eq!(config.server_listen_addr, "127.0.0.1");
        assert_eq!(config.server_listen_port, 6767);
        assert_eq!(config.server_max_connections, 1024);
        assert_eq!(config.server_security_mode, ServerSecurityMode::Insecure);
        assert!(config.server_tls_cert_path.is_none());
        assert!(config.server_tls_key_path.is_none());
        assert!(config.server_auth_secret.is_none());
    }

    #[test]
    fn default_compaction_settings_match_documented_values() {
        let config = Config::default();
        assert_eq!(config.compaction_min_tables_per_tier, 4);
        assert_eq!(config.compaction_size_ratio, 1.5);
    }

    #[test]
    fn default_wal_sync_mode_is_always() {
        assert!(matches!(
            Config::default().wal_sync_mode,
            WalSyncMode::Always
        ));
    }

    #[test]
    fn validate_rejects_empty_data_dir() {
        let config = Config {
            data_dir: PathBuf::new(),
            ..Config::default()
        };
        assert_invalid(config.validate(), "data_dir");
    }

    #[test]
    fn validate_rejects_error_if_exists_when_create_if_missing_is_false() {
        let config = Config {
            create_if_missing: false,
            error_if_exists: true,
            ..Config::default()
        };
        assert_invalid(config.validate(), "create_if_missing");
    }

    #[test]
    fn validate_accepts_create_if_missing_false_when_error_if_exists_is_false() {
        let config = Config {
            create_if_missing: false,
            error_if_exists: false,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_create_if_missing_true_with_error_if_exists_true() {
        let config = Config {
            create_if_missing: true,
            error_if_exists: true,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_zero_max_key_size() {
        let config = Config {
            max_key_size_bytes: 0,
            ..Config::default()
        };
        assert_invalid(config.validate(), "max_key_size_bytes");
    }

    #[test]
    fn validate_rejects_zero_max_value_size() {
        let config = Config {
            max_value_size_bytes: 0,
            ..Config::default()
        };
        assert_invalid(config.validate(), "max_value_size_bytes");
    }

    #[test]
    fn validate_rejects_key_and_value_sizes_whose_sum_overflows() {
        let config = Config {
            max_key_size_bytes: usize::MAX,
            max_value_size_bytes: 1,
            ..Config::default()
        };
        assert_invalid(config.validate(), "max_value_size_bytes");
    }

    #[test]
    fn validate_rejects_memtable_smaller_than_largest_entry() {
        let config = Config {
            max_key_size_bytes: 100,
            max_value_size_bytes: 900,
            memtable_size_bytes: 999,
            ..Config::default()
        };
        assert_invalid(config.validate(), "memtable_size_bytes");
    }

    #[test]
    fn validate_accepts_memtable_exactly_as_large_as_largest_entry() {
        let config = Config {
            max_key_size_bytes: 100,
            max_value_size_bytes: 900,
            memtable_size_bytes: 1000,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_zero_sstable_block_size() {
        let config = Config {
            sstable_block_size_bytes: 0,
            ..Config::default()
        };
        assert_invalid(config.validate(), "sstable_block_size_bytes");
    }

    #[test]
    fn validate_accepts_zero_bloom_bits_because_it_disables_filters() {
        let config = Config {
            sstable_bloom_bits_per_key: 0,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_bloom_bits_at_upper_limit_of_32() {
        let config = Config {
            sstable_bloom_bits_per_key: 32,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_bloom_bits_above_32() {
        let config = Config {
            sstable_bloom_bits_per_key: 33,
            ..Config::default()
        };
        assert_invalid(config.validate(), "sstable_bloom_bits_per_key");
    }

    #[test]
    fn validate_rejects_fewer_than_two_tables_per_compaction_tier() {
        let config = Config {
            compaction_min_tables_per_tier: 1,
            ..Config::default()
        };
        assert_invalid(config.validate(), "compaction_min_tables_per_tier");
    }

    #[test]
    fn validate_accepts_exactly_two_tables_per_compaction_tier() {
        let config = Config {
            compaction_min_tables_per_tier: 2,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_compaction_size_ratio_equal_to_one() {
        let config = Config {
            compaction_size_ratio: 1.0,
            ..Config::default()
        };
        assert_invalid(config.validate(), "compaction_size_ratio");
    }

    #[test]
    fn validate_rejects_compaction_size_ratio_below_one() {
        let config = Config {
            compaction_size_ratio: 0.5,
            ..Config::default()
        };
        assert_invalid(config.validate(), "compaction_size_ratio");
    }

    #[test]
    fn validate_rejects_nan_compaction_size_ratio() {
        let config = Config {
            compaction_size_ratio: f32::NAN,
            ..Config::default()
        };
        assert_invalid(config.validate(), "compaction_size_ratio");
    }

    #[test]
    fn validate_rejects_infinite_compaction_size_ratio() {
        let config = Config {
            compaction_size_ratio: f32::INFINITY,
            ..Config::default()
        };
        assert_invalid(config.validate(), "compaction_size_ratio");
    }

    #[test]
    fn validate_accepts_compaction_size_ratio_just_above_one() {
        let config = Config {
            compaction_size_ratio: 1.01,
            ..Config::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_server_accepts_ipv4_listen_address() {
        let config = Config {
            server_listen_addr: "0.0.0.0".to_string(),
            ..Config::default()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn validate_server_accepts_ipv6_listen_address() {
        let config = Config {
            server_listen_addr: "::1".to_string(),
            ..Config::default()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn validate_server_rejects_hostname_as_listen_address() {
        let config = Config {
            server_listen_addr: "localhost".to_string(),
            ..Config::default()
        };
        assert_invalid(config.validate_server(), "server_listen_addr");
    }

    #[test]
    fn validate_server_rejects_empty_listen_address() {
        let config = Config {
            server_listen_addr: String::new(),
            ..Config::default()
        };
        assert_invalid(config.validate_server(), "server_listen_addr");
    }

    #[test]
    fn validate_server_rejects_zero_max_connections() {
        let config = Config {
            server_max_connections: 0,
            ..Config::default()
        };
        assert_invalid(config.validate_server(), "server_max_connections");
    }

    #[test]
    fn validate_server_accepts_single_max_connection() {
        let config = Config {
            server_max_connections: 1,
            ..Config::default()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn validate_server_accepts_fully_specified_tls_config() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            ..tls_config()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn validate_server_rejects_tls_without_cert_path() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_tls_cert_path: None,
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_tls_cert_path");
    }

    #[test]
    fn validate_server_rejects_tls_with_empty_cert_path() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_tls_cert_path: Some(PathBuf::new()),
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_tls_cert_path");
    }

    #[test]
    fn validate_server_rejects_tls_without_key_path() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_tls_key_path: None,
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_tls_key_path");
    }

    #[test]
    fn validate_server_rejects_tls_with_empty_key_path() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_tls_key_path: Some(PathBuf::new()),
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_tls_key_path");
    }

    #[test]
    fn validate_server_rejects_tls_without_auth_secret() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_auth_secret: None,
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_auth_secret");
    }

    #[test]
    fn validate_server_rejects_tls_with_empty_auth_secret() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Tls,
            server_auth_secret: Some(String::new()),
            ..tls_config()
        };
        assert_invalid(config.validate_server(), "server_auth_secret");
    }

    #[test]
    fn validate_server_accepts_insecure_mode_without_tls_settings() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Insecure,
            ..Config::default()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn validate_server_accepts_insecure_mode_even_when_tls_settings_are_present() {
        let config = Config {
            server_security_mode: ServerSecurityMode::Insecure,
            ..tls_config()
        };
        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn load_returns_defaults_for_empty_file() {
        let path = write_config_file("empty_file", "");
        let config = Config::load(&path).expect("empty config file should load with defaults");
        assert_eq!(config.data_dir, Config::default().data_dir);
        assert_eq!(
            config.server_listen_port,
            Config::default().server_listen_port
        );
    }

    #[test]
    fn load_overrides_only_the_fields_present_in_file() {
        let path = write_config_file(
            "partial_override",
            "data_dir = \"/var/lib/mortdb\"\nserver_listen_port = 7000\n",
        );
        let config = Config::load(&path).expect("partial config file should load");
        assert_eq!(config.data_dir, PathBuf::from("/var/lib/mortdb"));
        assert_eq!(config.server_listen_port, 7000);
        assert_eq!(
            config.max_key_size_bytes,
            Config::default().max_key_size_bytes
        );
    }

    #[test]
    fn load_parses_security_mode_from_snake_case_string() {
        let path = write_config_file(
            "security_mode_tls",
            "server_security_mode = \"tls\"\n\
             server_tls_cert_path = \"cert.pem\"\n\
             server_tls_key_path = \"key.pem\"\n\
             server_auth_secret = \"secret\"\n",
        );
        let config = Config::load(&path).expect("tls config file should load");
        assert_eq!(config.server_security_mode, ServerSecurityMode::Tls);
        assert_eq!(config.server_tls_cert_path, Some(PathBuf::from("cert.pem")));
    }

    #[test]
    fn load_rejects_unknown_security_mode() {
        let path = write_config_file(
            "unknown_security_mode",
            "server_security_mode = \"quantum\"\n",
        );
        let result = Config::load(&path);
        assert!(matches!(
            result,
            Err(Error::InvalidConfig { field: None, .. })
        ));
    }

    #[test]
    fn load_rejects_unknown_field() {
        let path = write_config_file("unknown_field", "not_a_real_setting = 1\n");
        let result = Config::load(&path);
        assert!(matches!(
            result,
            Err(Error::InvalidConfig { field: None, .. })
        ));
    }

    #[test]
    fn load_rejects_malformed_toml() {
        let path = write_config_file("malformed_toml", "data_dir = = oops\n");
        let result = Config::load(&path);
        assert!(matches!(
            result,
            Err(Error::InvalidConfig { field: None, .. })
        ));
    }

    #[test]
    fn load_rejects_value_of_wrong_type() {
        let path = write_config_file("wrong_type", "server_listen_port = \"not a number\"\n");
        let result = Config::load(&path);
        assert!(matches!(
            result,
            Err(Error::InvalidConfig { field: None, .. })
        ));
    }

    #[test]
    fn load_runs_validation_and_rejects_semantically_invalid_values() {
        let path = write_config_file("invalid_value", "max_key_size_bytes = 0\n");
        assert_invalid(Config::load(&path), "max_key_size_bytes");
    }

    #[test]
    fn load_returns_io_error_when_file_does_not_exist() {
        let result = Config::load("/path/to/nonexistent/mortdb.toml");
        assert!(matches!(
            result,
            Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound
        ));
    }
}
