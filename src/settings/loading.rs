//! Configuration file loading for Settings.

use config::{Config, ConfigError, Environment, File};

use super::Settings;

impl Settings {
    /// Load settings from all available sources with proper precedence.
    ///
    /// Precedence (highest to lowest):
    /// 1. Values set explicitly via `with_overrides()`
    /// 2. Environment variables (`TOPCAT_*`)
    /// 3. Project config file (`./topcat.toml`)
    /// 4. User config file (`~/.config/topcat/config.toml`)
    /// 5. System config file (`/etc/topcat/config.toml`)
    /// 6. Default values
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional explicit config file path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use topcat::settings::Settings;
    ///
    /// // Load from standard locations + env vars
    /// let settings = Settings::load(None).unwrap();
    ///
    /// // Load from explicit config file
    /// let settings = Settings::load(Some("my-config.toml")).unwrap();
    /// ```
    pub fn load(config_path: Option<&str>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Add default configuration file locations (lowest priority)
        // System-wide config
        if let Ok(path) = Self::get_system_config_path() {
            builder = builder.add_source(File::with_name(&path).required(false));
        }

        // User config
        if let Ok(path) = Self::get_user_config_path() {
            builder = builder.add_source(File::with_name(&path).required(false));
        }

        // Project config (./topcat.toml or ./.topcat.toml)
        builder = builder
            .add_source(File::with_name("./topcat").required(false))
            .add_source(File::with_name("./.topcat").required(false));

        // Explicit config file path (if provided)
        if let Some(path) = config_path {
            builder = builder.add_source(File::with_name(path).required(true));
        }

        // Environment variables with TOPCAT_ prefix (higher priority)
        // Use __ as separator for nested config (e.g., TOPCAT_SQL_DISCOVERY__ENABLED)
        builder = builder.add_source(
            Environment::with_prefix("TOPCAT")
                .separator("__")
                .try_parsing(true)
                .list_separator(","),
        );

        // Build and deserialize
        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Get the system-wide config file path.
    pub(crate) fn get_system_config_path() -> Result<String, std::io::Error> {
        #[cfg(unix)]
        {
            Ok("/etc/topcat/config".to_string())
        }
        #[cfg(windows)]
        {
            Ok("C:\\ProgramData\\topcat\\config".to_string())
        }
    }

    /// Get the user config file path.
    pub(crate) fn get_user_config_path() -> Result<String, std::io::Error> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir
                .join("topcat")
                .join("config")
                .display()
                .to_string())
        } else if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir
                .join(".config")
                .join("topcat")
                .join("config")
                .display()
                .to_string())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine user config directory",
            ))
        }
    }
}
