use std::path::PathBuf;
use zed_extension_api as zed;

/// Convert relative path to absolute path, handling Windows separators
pub fn normalize_path_to_absolute(relative_path: &str) -> String {
    // Detect platform at runtime using Zed API
    let (platform, _) = zed::current_platform();

    match platform {
        zed::Os::Windows => {
            // If the path already starts with a drive letter (C:, D:, etc.), it's already absolute
            if relative_path.len() > 1 && relative_path.chars().nth(1) == Some(':') {
                // Already absolute, just normalize slashes
                return relative_path.replace('\\', "/");
            }

            // Convert to PathBuf
            let path_buf = PathBuf::from(relative_path);

            // Get absolute path by converting to absolute
            let absolute_path = if path_buf.is_absolute() {
                path_buf
            } else {
                // For relative paths, prepend current directory
                if let Ok(cwd) = std::env::current_dir() {
                    cwd.join(&path_buf)
                } else {
                    path_buf
                }
            };

            let mut path_str = absolute_path.to_string_lossy().to_string();

            // Fix the /C:/ prefix issue (convert to C:/)
            if path_str.starts_with('/') && path_str.len() > 2 && path_str.chars().nth(2) == Some(':') {
                path_str = path_str[1..].to_string();
            }

            // Convert all backslashes to forward slashes for consistency
            path_str = path_str.replace('\\', "/");

            path_str
        }
        _ => {
            // On Unix-like systems, just return as-is if absolute
            if relative_path.starts_with('/') {
                return relative_path.to_string();
            }

            // Convert to PathBuf
            let path_buf = PathBuf::from(relative_path);

            // Make absolute if needed
            let absolute_path = if path_buf.is_absolute() {
                path_buf
            } else {
                if let Ok(cwd) = std::env::current_dir() {
                    cwd.join(&path_buf)
                } else {
                    path_buf
                }
            };

            absolute_path.to_string_lossy().to_string()
        }
    }
}
