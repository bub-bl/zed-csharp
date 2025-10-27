use std::io::Write;
use std::sync::OnceLock;

pub struct Logger;

static LOGGER: OnceLock<Logger> = OnceLock::new();

impl Logger {
    /// Enable/disable debug logging - set to false for production
    const DEBUG_ENABLED: bool = true;

    pub fn instance() -> &'static Logger {
        LOGGER.get_or_init(|| Logger)
    }

    pub fn debug(message: &str) {
        Self::instance().debug_log(message);
    }

    pub fn info(message: &str) {
        Self::instance().info_log(message);
    }

    pub fn error(message: &str) {
        Self::instance().error_log(message);
    }

    pub fn warn(message: &str) {
        Self::instance().warn_log(message);
    }

    fn debug_log(&self, message: &str) {
        if !Self::DEBUG_ENABLED {
            return;
        }
        self.write_log("DEBUG", message);
    }

    fn info_log(&self, message: &str) {
        self.write_log("INFO", message);
    }

    fn error_log(&self, message: &str) {
        self.write_log("ERROR", message);
    }

    fn warn_log(&self, message: &str) {
        self.write_log("WARN", message);
    }

    fn write_log(&self, level: &str, message: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("csharp_extension_debug.log")
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message);
        }
    }
}
