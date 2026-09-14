use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    #[serde(default = "default_log_format")]
    pub log_format: LogFormat,
}

fn default_log_format() -> LogFormat {
    LogFormat::Pretty
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to load configuration: {0}")]
    Load(#[from] config::ConfigError),
}

impl AppConfig {
    /// Loads configuration from environment variables prefixed with `APP__`,
    /// e.g. `APP__BIND_ADDR`, `APP__DATABASE_URL`, `APP__LOG_FORMAT`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let settings = config::Config::builder()
            .set_default("log_format", "pretty")?
            .add_source(
                config::Environment::default()
                    .prefix("APP")
                    .separator("__"),
            )
            .build()?;

        Ok(settings.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        unsafe {
            std::env::remove_var("APP__BIND_ADDR");
            std::env::remove_var("APP__DATABASE_URL");
            std::env::remove_var("APP__LOG_FORMAT");
        }
    }

    #[test]
    #[serial]
    fn from_env_reads_all_fields_from_environment() {
        clear_env();
        unsafe {
            std::env::set_var("APP__BIND_ADDR", "0.0.0.0:8080");
            std::env::set_var("APP__DATABASE_URL", "postgres://user:pass@localhost/trovr");
            std::env::set_var("APP__LOG_FORMAT", "json");
        }

        let config = AppConfig::from_env().expect("config should load");

        assert_eq!(config.bind_addr, "0.0.0.0:8080");
        assert_eq!(config.database_url, "postgres://user:pass@localhost/trovr");
        assert_eq!(config.log_format, LogFormat::Json);

        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_defaults_log_format_to_pretty_when_unset() {
        clear_env();
        unsafe {
            std::env::set_var("APP__BIND_ADDR", "0.0.0.0:8080");
            std::env::set_var("APP__DATABASE_URL", "postgres://user:pass@localhost/trovr");
        }

        let config = AppConfig::from_env().expect("config should load");

        assert_eq!(config.log_format, LogFormat::Pretty);

        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_errors_when_required_fields_are_missing() {
        clear_env();

        let result = AppConfig::from_env();

        assert!(result.is_err());
    }
}
