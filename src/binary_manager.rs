use std::fs;
use std::io::Cursor;
use zed_extension_api::{self as zed, http_client, Result};

use crate::logger;
use crate::path_utils;
use crate::path_utils::normalize_path_to_absolute;
use crate::version_config::VersionDirConfig;

pub struct BinaryManager {
    cached_version_dir: Option<String>,
}

impl BinaryManager {
    pub fn new() -> Self {
        Self {
            cached_version_dir: None,
        }
    }

    /// Download file from URL using HTTP
    fn download_file_http(url: &str) -> Result<Vec<u8>> {
        logger::Logger::debug(&format!("download_file_http: downloading from {}", url));

        let request = http_client::HttpRequest {
            method: http_client::HttpMethod::Get,
            url: url.to_string(),
            headers: Default::default(),
            body: None,
            redirect_policy: http_client::RedirectPolicy::FollowAll,
        };

        let response =
            http_client::fetch(&request).map_err(|e| format!("HTTP fetch failed: {}", e))?;

        logger::Logger::debug(&format!(
            "download_file_http: received {} bytes",
            response.body.len()
        ));

        // Check if we actually got data
        if response.body.is_empty() {
            return Err("downloaded file is empty".to_string());
        }

        Ok(response.body)
    }

    /// Extract ZIP file using the zip crate (pure Rust, no C dependencies)
    fn extract_zip(zip_data: &[u8], destination: &str) -> Result<()> {
        logger::Logger::debug(&format!(
            "extract_zip: extracting {} bytes to {}",
            zip_data.len(),
            destination
        ));

        // Ensure destination directory exists
        fs::create_dir_all(destination)
            .map_err(|e| format!("failed to create destination directory: {}", e))?;

        // Extract ZIP from memory
        let cursor = Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("failed to open zip archive: {}", e))?;

        logger::Logger::debug(&format!(
            "extract_zip: archive has {} entries",
            archive.len()
        ));

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("failed to read zip entry {}: {}", i, e))?;

            // Get the file path, handling various path formats
            let file_path_str = if let Some(enclosed_name) = file.enclosed_name() {
                enclosed_name.to_string_lossy().to_string()
            } else {
                logger::Logger::warn(&format!(
                    "extract_zip: skipping entry {} with invalid path",
                    i
                ));
                continue;
            };

            if file_path_str.is_empty() {
                continue;
            }

            let outpath = std::path::PathBuf::from(destination).join(&file_path_str);

            logger::Logger::debug(&format!(
                "extract_zip: processing entry: {} (size: {} bytes, is_dir: {})",
                file_path_str,
                file.size(),
                file.is_dir()
            ));

            if file.is_dir() {
                // Directory entry
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("failed to create directory {}: {}", file_path_str, e))?;
            } else {
                // File entry
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        format!(
                            "failed to create parent directory for {}: {}",
                            file_path_str, e
                        )
                    })?;
                }

                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("failed to create file {}: {}", file_path_str, e))?;

                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("failed to copy file {}: {}", file_path_str, e))?;

                logger::Logger::debug(&format!(
                    "extract_zip: successfully extracted {} ({} bytes)",
                    file_path_str,
                    file.size()
                ));
            }
        }

        logger::Logger::debug("extract_zip: extraction completed successfully");
        Ok(())
    }

    fn download_with_retry(url: &str, destination: &str, max_retries: usize) -> Result<()> {
        let mut attempt = 0;

        while attempt < max_retries {
            attempt += 1;
            logger::Logger::debug(&format!(
                "download_with_retry: attempting download (attempt {}/{})",
                attempt, max_retries
            ));

            // use custom HTTP download and ZIP extraction
            let result = {
                let zip_data = match Self::download_file_http(url) {
                    Ok(data) => data,
                    Err(e) => {
                        logger::Logger::warn(&format!(
                            "download_with_retry: attempt {} download failed: {}",
                            attempt, e
                        ));

                        if attempt < max_retries {
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                };

                Self::extract_zip(&zip_data, destination)
            };

            match result {
                Ok(()) => {
                    logger::Logger::debug("download_with_retry: download/extraction succeeded");
                    return Ok(());
                }
                Err(e) => {
                    let error_str = e.to_string();
                    logger::Logger::warn(&format!(
                        "download_with_retry: attempt {} failed: {}",
                        attempt, error_str
                    ));

                    // Check if this is a retryable error
                    let is_retryable = error_str.contains("unexpected end of file")
                        || error_str.contains("extraction")
                        || error_str.contains("download")
                        || error_str.contains("incomplete write")
                        || error_str.contains("failed");

                    if is_retryable && attempt < max_retries {
                        // Clean up the corrupted directory before retrying
                        logger::Logger::debug(&format!(
                            "download_with_retry: cleaning up corrupted directory before retry"
                        ));
                        fs::remove_dir_all(destination).ok();
                        fs::create_dir_all(destination).map_err(|e| {
                            format!("failed to create directory {}: {}", destination, e)
                        })?;

                        logger::Logger::debug(&format!(
                            "download_with_retry: retrying download (attempt {} of {})",
                            attempt + 1,
                            max_retries
                        ));
                    } else {
                        // Either not retryable or max retries exhausted
                        return Err(error_str);
                    }
                }
            }
        }

        // Should not reach here, but return error if we do
        Err("download retries exhausted".to_string())
    }

    /// Get the version directory, downloading if necessary
    pub fn get_version_dir(
        &mut self,
        config: &VersionDirConfig,
        language_server_id: Option<&zed::LanguageServerId>,
    ) -> Result<String> {
        let fn_name = format!("get_version_dir[{}]", config.prefix);
        logger::Logger::debug(&format!("{}: starting version check", fn_name));

        // First check if we have a cached version directory
        if let Some(cached_dir) = &self.cached_version_dir {
            if fs::metadata(cached_dir).map_or(false, |stat| stat.is_dir()) {
                logger::Logger::debug(&format!(
                    "{}: using cached version directory: {}",
                    fn_name, cached_dir
                ));
                return Ok(cached_dir.clone());
            }
        }

        // Try to find the latest local version first
        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
        let mut latest_local_version = None;

        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&config.prefix)
                    && fs::metadata(&name).map_or(false, |stat| stat.is_dir())
                {
                    let version = name.trim_start_matches(&format!("{}-", config.prefix));
                    if latest_local_version
                        .as_ref()
                        .map_or(true, |latest: &String| version > latest)
                    {
                        latest_local_version = Some(version.to_string());
                    }
                }
            }
        }

        // Check GitHub for updates if we can
        if let Some(language_server_id) = language_server_id {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );
        }

        let github_version = zed::latest_github_release(
            &config.github_repo,
            zed::GithubReleaseOptions {
                require_assets: false,
                pre_release: false,
            },
        )
        .ok()
        .map(|release| release.version.trim_start_matches('v').to_string());

        // Use GitHub version if it's newer than local, otherwise use local
        let version = if let Some(gh_ver) = github_version {
            if latest_local_version
                .as_ref()
                .map_or(true, |local| gh_ver > *local)
            {
                gh_ver
            } else {
                latest_local_version.unwrap()
            }
        } else {
            // No GitHub access, fall back to local version
            latest_local_version.ok_or_else(|| {
                format!(
                    "No {} version found locally and cannot check GitHub for updates",
                    config.prefix
                )
            })?
        };

        let version_dir = format!("{}-{}", config.prefix, version);

        // If we already have this version locally, validate it's complete
        if fs::metadata(&version_dir).map_or(false, |stat| stat.is_dir()) {
            // Check if the expected binary exists to validate the download was complete
            let binary_path = (config.get_binary_path)(&version_dir);

            if fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!(
                    "{}: validated existing directory: {}",
                    fn_name, version_dir
                ));
                if let Some(language_server_id) = language_server_id {
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                }
                // Convert to absolute path before caching and returning
                let absolute_version_dir = path_utils::normalize_path_to_absolute(&version_dir);
                self.cached_version_dir = Some(absolute_version_dir.clone());
                return Ok(absolute_version_dir);
            } else {
                // Directory exists but is incomplete/corrupted, clean it up
                logger::Logger::warn(&format!(
                    "{}: found incomplete directory, removing: {}",
                    fn_name, version_dir
                ));
                fs::remove_dir_all(&version_dir).ok();
            }
        }

        // Need to download new version
        let platform_str = (config.get_platform_string)()
            .map_err(|e| format!("{}: failed to determine platform: {}", fn_name, e))?;

        // Start download
        if let Some(language_server_id) = language_server_id {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
        }

        // Create the version directory if it doesn't exist
        fs::create_dir_all(&version_dir)
            .map_err(|e| format!("failed to create version directory {}: {}", version_dir, e))?;
        logger::Logger::debug(&format!("{}: created directory: {}", fn_name, version_dir));

        // Determine download URL using the config's resolver
        let download_url = (config.get_download_url)(&version, &platform_str)?;

        logger::Logger::debug(&format!("{}: downloading from {}", fn_name, download_url));

        // Use retry logic to download - handles incomplete downloads and extraction failures
        Self::download_with_retry(&download_url, &version_dir, 3)?;

        // Poll for the binary to appear (handles antivirus/file locker delays)
        let binary_path = (config.get_binary_path)(&version_dir);

        let max_polls = 50; // Poll up to 50 times
        let mut poll_count = 0;
        let mut has_content = fs::metadata(&binary_path).map_or(false, |stat| stat.is_file());

        while !has_content && poll_count < max_polls {
            poll_count += 1;
            has_content = fs::metadata(&binary_path).map_or(false, |stat| stat.is_file());
            if !has_content && poll_count % 5 == 0 {
                logger::Logger::debug(&format!(
                    "{}: polling for {} binary... (poll {}/{})",
                    fn_name, config.binary_name_for_logging, poll_count, max_polls
                ));
            }
        }

        if !has_content {
            logger::Logger::error(&format!(
                "{}: {} binary not found at {} after {} polls",
                fn_name, config.binary_name_for_logging, binary_path, poll_count
            ));
            fs::remove_dir_all(&version_dir).ok();
            return Err(format!(
                "failed to download {}: binary not found after extraction",
                config.prefix
            ));
        }

        // Validate the binary is a valid Windows PE executable
        #[cfg(windows)]
        {
            if let Ok(metadata) = fs::metadata(&binary_path) {
                let file_size = metadata.len();
                logger::Logger::debug(&format!(
                    "{}: {} binary size: {} bytes",
                    fn_name, config.binary_name_for_logging, file_size
                ));

                // Check if file is large enough to be a valid PE executable (min ~100KB)
                if file_size < 100_000 {
                    logger::Logger::error(&format!(
                        "{}: {} binary appears too small: {} bytes",
                        fn_name, config.binary_name_for_logging, file_size
                    ));
                    // Don't fail yet, it might still work
                }

                // Try to read first few bytes to check for PE signature
                if let Ok(mut file) = fs::File::open(&binary_path) {
                    use std::io::Read;
                    let mut header = [0u8; 2];
                    if file.read_exact(&mut header).is_ok() {
                        if header == [0x4d, 0x5a] {
                            // "MZ" - DOS header for PE files
                            logger::Logger::debug(&format!(
                                "{}: {} binary has valid PE header",
                                fn_name, config.binary_name_for_logging
                            ));
                        } else {
                            logger::Logger::error(&format!(
                                "{}: {} binary has invalid header: {:02x}{:02x}",
                                fn_name, config.binary_name_for_logging, header[0], header[1]
                            ));
                            fs::remove_dir_all(&version_dir).ok();
                            return Err(format!(
                                "{} binary has invalid PE header signature",
                                config.binary_name_for_logging
                            ));
                        }
                    }
                }
            }
        }

        logger::Logger::debug(&format!(
            "{}: successfully downloaded and extracted to {} (polls: {})",
            fn_name, version_dir, poll_count
        ));

        // Clean up old versions
        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}-", config.prefix)) && name != version_dir {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        if let Some(language_server_id) = language_server_id {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );
        }

        // Convert to absolute path before caching and returning
        let absolute_version_dir = path_utils::normalize_path_to_absolute(&version_dir);
        self.cached_version_dir = Some(absolute_version_dir.clone());
        Ok(absolute_version_dir)
    }
}
