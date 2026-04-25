use crate::{
    discovery::{DEFAULT_INITIAL_PUBLISH_DELAY, DEFAULT_REPUBLISH_DELAY},
    fs::FS_PATH,
};
use log::error;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

pub(crate) static CONFIG_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("config.toml"));

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configuration of an Oku file system node.
pub struct OkuFsConfig {
    /// The delay between republishing content to the Mainline DHT (defaults to [`crate::discovery::DEFAULT_REPUBLISH_DELAY`]).
    republish_delay: Arc<Mutex<Option<Duration>>>,
    /// The initial delay before publishing content to the Mainline DHT (defaults to [`crate::discovery::DEFAULT_INITIAL_PUBLISH_DELAY`]).
    initial_publish_delay: Arc<Mutex<Option<Duration>>>,
}

impl Default for OkuFsConfig {
    fn default() -> Self {
        Self {
            republish_delay: Arc::new(Mutex::new(None)),
            initial_publish_delay: Arc::new(Mutex::new(None)),
        }
    }
}

impl OkuFsConfig {
    /// Loads the configuration of the file system from disk, or creates a new configuration if none exists.
    ///
    /// # Returns
    ///
    /// The configuration of the file system.
    pub fn load_or_create_config() -> miette::Result<Self> {
        let config_file_contents = std::fs::read_to_string(&*CONFIG_PATH);
        match config_file_contents {
            Ok(config_file_toml) => match toml::from_str(&config_file_toml) {
                Ok(config) => Ok(config),
                Err(e) => {
                    error!("{}", e);
                    let config = Self::default();
                    Ok(config)
                }
            },
            Err(e) => {
                error!("{}", e);
                let config = Self::default();
                let config_toml = toml::to_string_pretty(&config).into_diagnostic()?;
                std::fs::write(&*CONFIG_PATH, config_toml).into_diagnostic()?;
                Ok(config)
            }
        }
    }

    /// Writes the configuration to disk.
    pub fn save(&self) -> miette::Result<()> {
        let config_toml = toml::to_string_pretty(&self).into_diagnostic()?;
        std::fs::write(&*CONFIG_PATH, config_toml).into_diagnostic()?;
        Ok(())
    }

    /// Gets [`OkuFsConfig::republish_delay`].
    ///
    /// # Returns
    ///
    /// [`OkuFsConfig::republish_delay`] if set, or [`crate::discovery::DEFAULT_REPUBLISH_DELAY`] otherwise.
    pub fn get_republish_delay(&self) -> Duration {
        self.republish_delay
            .try_lock()
            .ok()
            .map(|x| x.to_owned())
            .flatten()
            .unwrap_or(DEFAULT_REPUBLISH_DELAY)
    }

    /// Sets [`OkuFsConfig::republish_delay`].
    ///
    /// # Arguments
    ///
    /// * `republish_delay` - An optional republish delay; if unspecified, the default will be used.
    pub fn set_republish_delay(&self, republish_delay: &Option<Duration>) -> miette::Result<()> {
        *self
            .republish_delay
            .try_lock()
            .map_err(|e| miette!("{}", e))? = *republish_delay;
        Ok(())
    }

    /// Gets [`OkuFsConfig::initial_publish_delay`].
    ///
    /// # Returns
    ///
    /// [`OkuFsConfig::initial_publish_delay`] if set, or [`crate::discovery::DEFAULT_INITIAL_PUBLISH_DELAY`] otherwise.
    pub fn get_initial_publish_delay(&self) -> Duration {
        self.initial_publish_delay
            .try_lock()
            .ok()
            .map(|x| x.to_owned())
            .flatten()
            .unwrap_or(DEFAULT_INITIAL_PUBLISH_DELAY)
    }

    /// Sets [`OkuFsConfig::initial_publish_delay`].
    ///
    /// # Arguments
    ///
    /// * `initial_publish_delay` - An optional initial publish delay; if unspecified, the default will be used.
    pub fn set_initial_publish_delay(
        &self,
        initial_publish_delay: &Option<Duration>,
    ) -> miette::Result<()> {
        *self
            .initial_publish_delay
            .try_lock()
            .map_err(|e| miette!("{}", e))? = *initial_publish_delay;
        Ok(())
    }
}
