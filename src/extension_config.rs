use crate::logger;
use zed_extension_api::{serde_json, settings::LspSettings, Worktree};

/// Configuration for the C# extension
#[derive(Debug, Clone)]
pub struct ExtensionConfig {
    /// Enable broad search for solution files (searches parent directories)
    pub broad_search: bool,
    /// Log level for Roslyn (Debug, Information, Warning, Error)
    pub log_level: String,
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            broad_search: false,
            log_level: "Information".to_string(),
        }
    }
}

impl ExtensionConfig {
    /// Load configuration from Zed settings
    pub fn load(worktree: Option<&Worktree>) -> Self {
        let mut config = Self::default();

        // Try to load from Zed settings
        if let Some(wt) = worktree {
            if let Ok(settings) = LspSettings::for_worktree("csharp-roslyn", wt) {
                if let Some(settings_obj) = settings.settings {
                    // Try to parse custom extension settings
                    if let Ok(custom_settings) = serde_json::from_value::<
                        serde_json::Map<String, serde_json::Value>,
                    >(settings_obj)
                    {
                        // Load broad_search setting
                        if let Some(broad_search) = custom_settings.get("broad_search") {
                            if let Some(val) = broad_search.as_bool() {
                                config.broad_search = val;
                                logger::Logger::debug(&format!(
                                    "ExtensionConfig: loaded broad_search = {}",
                                    val
                                ));
                            }
                        }

                        // Load log_level setting
                        if let Some(log_level) = custom_settings.get("log_level") {
                            if let Some(val) = log_level.as_str() {
                                config.log_level = val.to_string();
                                logger::Logger::debug(&format!(
                                    "ExtensionConfig: loaded log_level = {}",
                                    val
                                ));
                            }
                        }
                    }
                }
            }
        }

        logger::Logger::debug(&format!("ExtensionConfig: using config {:?}", config));

        config
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate log level
        match self.log_level.as_str() {
            "Debug" | "Information" | "Warning" | "Error" => {}
            other => {
                return Err(format!(
                    "Invalid log_level '{}'. Expected one of: Debug, Information, Warning, Error",
                    other
                ))
            }
        }

        Ok(())
    }
}
