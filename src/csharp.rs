mod logger;

use std::fs;
use std::io::Cursor;
use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    settings::LspSettings,
    http_client,
    DebugAdapterBinary, DebugTaskDefinition, LanguageServerId, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

struct CsharpExtension {
    cached_roslyn_path: Option<String>,
    cached_razor_path: Option<String>,
    cached_debugger_path: Option<String>,
    cached_version_dir: Option<String>,
}

impl CsharpExtension {
    /// Convert relative path to absolute path, handling Windows separators
    fn normalize_path_to_absolute(relative_path: &str) -> String {
        use std::path::PathBuf;
        
        // Convert to PathBuf
        let path_buf = PathBuf::from(relative_path);
        
        // Detect platform at runtime using Zed API
        let (platform, _) = zed::current_platform();
        
        match platform {
            zed::Os::Windows => {
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
                
                // Convert forward slashes to backslashes
                path_str = path_str.replace('/', "\\");
                
                // Strip leading backslash if it's followed by a drive letter (e.g., \C: -> C:)
                if path_str.starts_with("\\") && path_str.len() > 2 && path_str.chars().nth(2) == Some(':') {
                    path_str = path_str[1..].to_string();
                }
                
                path_str
            }
            _ => {
                // On Unix-like systems, make absolute if needed
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

    /// Download file from URL using HTTP (Windows only)
    fn download_file_http(url: &str) -> Result<Vec<u8>> {
        logger::Logger::debug(&format!("download_file_http: downloading from {}", url));
        
        let request = http_client::HttpRequest {
            method: http_client::HttpMethod::Get,
            url: url.to_string(),
            headers: Default::default(),
            body: None,
            redirect_policy: http_client::RedirectPolicy::FollowAll,
        };

        let response = http_client::fetch(&request)
            .map_err(|e| format!("HTTP fetch failed: {}", e))?;

        logger::Logger::debug(&format!("download_file_http: received {} bytes", response.body.len()));
        
        // Check if we actually got data
        if response.body.is_empty() {
            return Err("downloaded file is empty".to_string());
        }
        
        Ok(response.body)
    }

    /// Extract ZIP file using the zip crate (pure Rust, no C dependencies)
    fn extract_zip(zip_data: &[u8], destination: &str) -> Result<()> {
        logger::Logger::debug(&format!("extract_zip: extracting {} bytes to {}", zip_data.len(), destination));
        
        // Ensure destination directory exists
        fs::create_dir_all(destination)
            .map_err(|e| format!("failed to create destination directory: {}", e))?;

        // Extract ZIP from memory
        let cursor = Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("failed to open zip archive: {}", e))?;

        logger::Logger::debug(&format!("extract_zip: archive has {} entries", archive.len()));

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("failed to read zip entry {}: {}", i, e))?;

            // Get the file path, handling various path formats
            let file_path_str = if let Some(enclosed_name) = file.enclosed_name() {
                enclosed_name.to_string_lossy().to_string()
            } else {
                logger::Logger::warn(&format!("extract_zip: skipping entry {} with invalid path", i));
                continue;
            };

            if file_path_str.is_empty() {
                continue;
            }

            let outpath = std::path::PathBuf::from(destination).join(&file_path_str);

            logger::Logger::debug(&format!("extract_zip: processing entry: {} (size: {} bytes, is_dir: {})", 
                file_path_str, file.size(), file.is_dir()));

            if file.is_dir() {
                // Directory entry
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("failed to create directory {}: {}", file_path_str, e))?;
            } else {
                // File entry
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("failed to create parent directory for {}: {}", file_path_str, e))?;
                }
                
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("failed to create file {}: {}", file_path_str, e))?;
                
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("failed to copy file {}: {}", file_path_str, e))?;

                logger::Logger::debug(&format!("extract_zip: successfully extracted {} ({} bytes)", 
                    file_path_str, file.size()));
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
                            attempt + 1, max_retries
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

    

    fn get_vscode_version_dir(&mut self, language_server_id: Option<&LanguageServerId>) -> Result<String> {
        logger::Logger::debug("get_vscode_version_dir: starting version check");

        // First check if we have a cached version directory
        if let Some(cached_dir) = &self.cached_version_dir {
            if fs::metadata(cached_dir).map_or(false, |stat| stat.is_dir()) {
                logger::Logger::debug(&format!(
                    "get_vscode_version_dir: using cached version directory: {}",
                    cached_dir
                ));
                return Ok(cached_dir.clone());
            }
        }

        // Try to find the latest local version first
        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
        let mut latest_local_version = None;
        let prefix = "vscode-csharp-";

        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(prefix)
                    && fs::metadata(&name).map_or(false, |stat| stat.is_dir())
                {
                    let version = name.trim_start_matches(prefix);
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
            "dotnet/vscode-csharp",
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
                "No C# extension version found locally and cannot check GitHub for updates"
                    .to_string()
            })?
        };

        let version_dir = format!("vscode-csharp-{}", version);

        // If we already have this version locally, validate it's complete
        if fs::metadata(&version_dir).map_or(false, |stat| stat.is_dir()) {
            // Check if the expected Roslyn binary exists to validate the download was complete
            let (platform, _) = zed::current_platform();
            let binary_name = match platform {
                zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
                _ => "Microsoft.CodeAnalysis.LanguageServer",
            };
            let roslyn_binary = format!("{}/extension/.roslyn/{}", version_dir, binary_name);

            if fs::metadata(&roslyn_binary).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!(
                    "get_vscode_version_dir: validated existing directory: {}",
                    version_dir
                ));
                if let Some(language_server_id) = language_server_id {
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                }
                return Ok(version_dir);
            } else {
                // Directory exists but is incomplete/corrupted, clean it up
                logger::Logger::warn(&format!(
                    "get_vscode_version_dir: found incomplete directory, removing: {}",
                    version_dir
                ));
                fs::remove_dir_all(&version_dir).ok();
            }
        }

        // Need to download new version
        let (platform, arch) = zed::current_platform();
        let platform_str = match platform {
            zed::Os::Mac => match arch {
                zed::Architecture::Aarch64 => "darwin-arm64",
                _ => "darwin-x64",
            },
            zed::Os::Linux => match arch {
                zed::Architecture::Aarch64 => "linux-arm64",
                _ => "linux-x64",
            },
            zed::Os::Windows => match arch {
                zed::Architecture::Aarch64 => "win32-arm64",
                _ => "win32-x64",
            },
        };

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
        logger::Logger::debug(&format!(
            "get_vscode_version_dir: created directory: {}",
            version_dir
        ));

        let vsix_url = format!(
            "https://ms-dotnettools.gallery.vsassets.io/_apis/public/gallery/publisher/ms-dotnettools/extension/csharp/{}/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage?redirect=true&targetPlatform={}",
            version, platform_str
        );

        logger::Logger::debug(&format!("get_vscode_version_dir: downloading from {}", vsix_url));

        // Use retry logic to download - handles incomplete downloads and extraction failures
        Self::download_with_retry(&vsix_url, &version_dir, 3)?;

        // Poll for the binary to appear (handles antivirus/file locker delays)
        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };
        let roslyn_binary = format!("{}/extension/.roslyn/{}", version_dir, binary_name);

        let max_polls = 50; // Poll up to 50 times
        let mut poll_count = 0;
        let mut has_content = fs::metadata(&roslyn_binary).map_or(false, |stat| stat.is_file());

        while !has_content && poll_count < max_polls {
            poll_count += 1;
            has_content = fs::metadata(&roslyn_binary).map_or(false, |stat| stat.is_file());
            if !has_content && poll_count % 5 == 0 {
                logger::Logger::debug(&format!(
                    "get_vscode_version_dir: polling for Roslyn binary... (poll {}/{})",
                    poll_count, max_polls
                ));
            }
        }

        if !has_content {
            logger::Logger::error(&format!(
                "get_vscode_version_dir: Roslyn binary not found at {} after {} polls",
                roslyn_binary, poll_count
            ));
            fs::remove_dir_all(&version_dir).ok();
            return Err(format!(
                "failed to download VS Code C# extension: Roslyn binary not found after extraction"
            ));
        }

        // Validate the binary is a valid Windows PE executable
        #[cfg(windows)]
        {
            if let Ok(metadata) = fs::metadata(&roslyn_binary) {
                let file_size = metadata.len();
                logger::Logger::debug(&format!("get_vscode_version_dir: Roslyn binary size: {} bytes", file_size));
                
                // Check if file is large enough to be a valid PE executable (min ~100KB)
                if file_size < 100_000 {
                    logger::Logger::error(&format!("get_vscode_version_dir: Roslyn binary appears too small: {} bytes", file_size));
                    // Don't fail yet, it might still work
                }
                
                // Try to read first few bytes to check for PE signature
                if let Ok(mut file) = fs::File::open(&roslyn_binary) {
                    use std::io::Read;
                    let mut header = [0u8; 2];
                    if file.read_exact(&mut header).is_ok() {
                        if header == [0x4d, 0x5a] {  // "MZ" - DOS header for PE files
                            logger::Logger::debug("get_vscode_version_dir: Roslyn binary has valid PE header");
                        } else {
                            logger::Logger::error(&format!("get_vscode_version_dir: Roslyn binary has invalid header: {:02x}{:02x}", header[0], header[1]));
                            fs::remove_dir_all(&version_dir).ok();
                            return Err(format!("Roslyn binary has invalid PE header signature"));
                        }
                    }
                }
            }
        }

        logger::Logger::debug(&format!(
            "get_vscode_version_dir: successfully downloaded and extracted to {} (polls: {})",
            version_dir, poll_count
        ));

        // Clean up old versions
        let entries =
            fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("vscode-csharp-") && name != version_dir {
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

        self.cached_version_dir = Some(version_dir.clone());
        Ok(version_dir)
    }

    fn get_roslyn_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        logger::Logger::debug("get_roslyn_path: starting Roslyn path resolution");

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        let binary_settings = LspSettings::for_worktree("csharp-roslyn", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            logger::Logger::debug(&format!(
                "get_roslyn_path: using user-defined path: {}",
                path
            ));
            let absolute_path = Self::normalize_path_to_absolute(&path);
            return Ok(absolute_path);
        }

        // check for cached roslyn path
        if let Some(path) = &self.cached_roslyn_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_roslyn_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_roslyn_path: downloading extension");
        let version_dir = self.get_vscode_version_dir(Some(language_server_id))?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };

        let roslyn_path = format!("{}/extension/.roslyn/{}", version_dir, binary_name);

        if !fs::metadata(&roslyn_path).map_or(false, |stat| stat.is_file()) {
            return Err(format!(
                "Roslyn language server not found at: {}",
                roslyn_path
            ));
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&roslyn_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&roslyn_path, perms).ok();
            }
        }

        // Convert to absolute path for Windows compatibility (only for return to Zed)
        let absolute_path = Self::normalize_path_to_absolute(&roslyn_path);

        self.cached_roslyn_path = Some(absolute_path.clone());
        Ok(absolute_path)
    }

    fn get_razor_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        logger::Logger::debug("get_razor_path: starting");

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        let binary_settings = LspSettings::for_worktree("csharp-razor", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            logger::Logger::debug(&format!(
                "get_razor_path: using user-defined path: {}",
                path
            ));
            let absolute_path = Self::normalize_path_to_absolute(&path);
            return Ok(absolute_path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_razor_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_razor_path: resolving version directory for download");
        let version_dir = self.get_vscode_version_dir(Some(language_server_id))?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "rzls.exe",
            _ => "rzls",
        };

        let razor_path = format!("{}/extension/.razor/{}", version_dir, binary_name);
        logger::Logger::debug(&format!("get_razor_path: expected path: {}", razor_path));

        if !fs::metadata(&razor_path).map_or(false, |stat| stat.is_file()) {
            let error_msg = format!("Razor language server not found at: {}", razor_path);
            logger::Logger::error(&error_msg);
            return Err(error_msg);
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&razor_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&razor_path, perms).ok();
            }
        }

        // Convert to absolute path for Windows compatibility (only for return to Zed)
        let absolute_path = Self::normalize_path_to_absolute(&razor_path);

        logger::Logger::debug(&format!("get_razor_path: found at {}", absolute_path));
        self.cached_razor_path = Some(absolute_path.clone());
        Ok(absolute_path)
    }

    fn get_debugger_path(&mut self, user_provided_path: Option<String>) -> Result<String, String> {
        logger::Logger::debug("get_debugger_path: starting debugger path resolution");

        // check for user-defined path first
        if let Some(user_path) = user_provided_path {
            logger::Logger::debug(&format!(
                "get_debugger_path: using user-provided path: {}",
                user_path
            ));
            return Ok(user_path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!(
                    "get_debugger_path: using cached debugger path: {}",
                    path
                ));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_debugger_path: getting version directory");
        let version_dir = self.get_vscode_version_dir(None)?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "vsdbg.exe",
            _ => "vsdbg",
        };

        // windows debugger is in a platform subfolder
        let debugger_path = match platform {
            zed::Os::Windows => {
                format!("{}/extension/.debugger/x86_64/{}", version_dir, binary_name)
            }
            _ => format!("{}/extension/.debugger/{}", version_dir, binary_name),
        };

        if !fs::metadata(&debugger_path).map_or(false, |stat| stat.is_file()) {
            return Err(format!(
                "csharp debug server not found at: {}",
                debugger_path
            ));
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&debugger_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&debugger_path, perms).ok();
            }
        }

        // Convert to absolute path for Windows compatibility (only for return to Zed)
        let absolute_path = Self::normalize_path_to_absolute(&debugger_path);

        self.cached_debugger_path = Some(absolute_path.clone());
        Ok(absolute_path)
    }
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            cached_roslyn_path: None,
            cached_razor_path: None,
            cached_debugger_path: None,
            cached_version_dir: None,
        }
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        logger::Logger::debug(&format!(
            "get_dap_binary: requested adapter: {}",
            adapter_name
        ));

        if adapter_name != "coreclr" {
            logger::Logger::error(&format!(
                "get_dap_binary: unsupported adapter: {}",
                adapter_name
            ));
            return Err(format!("Cannot create binary for adapter: {adapter_name}"));
        }

        let configuration = config.config.to_string();

        let debugger_path = self
            .get_debugger_path(user_provided_debug_adapter_path)
            .map_err(|e| {
                logger::Logger::error(&format!("get_dap_binary: failed to locate debugger: {}", e));
                format!("Failed to locate C# debugger: {}", e)
            })?;

        logger::Logger::debug(&format!(
            "get_dap_binary: using debugger at: {}",
            debugger_path
        ));

        let request = if configuration.contains("\"request\":\"launch\"") {
            StartDebuggingRequestArgumentsRequest::Launch
        } else {
            StartDebuggingRequestArgumentsRequest::Attach
        };

        Ok(DebugAdapterBinary {
            command: Some(debugger_path),
            arguments: vec!["--interpreter=vscode".to_string()],
            envs: Default::default(),
            cwd: Some(worktree.root_path()),
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration,
                request,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        if adapter_name != "coreclr" {
            return Err(format!("Unknown adapter: {}", adapter_name));
        }

        match config.get("request").and_then(|v| v.as_str()) {
            Some("launch") => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            Some("attach") => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            Some(other) => Err(format!(
                "Invalid 'request' value: '{}'. Expected 'launch' or 'attach'",
                other
            )),
            None => Err(
                "Debug configuration missing required 'request' field. Must be 'launch' or 'attach'"
                    .to_string(),
            ),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Parse the language server ID to determine which server to use
        let server_id_str = format!("{:?}", language_server_id);
        logger::Logger::debug(&format!(
            "language_server_command: server_id: {}",
            server_id_str
        ));

        match server_id_str.as_str() {
            s if s.contains("rzls") => {
                // Razor Language Server
                logger::Logger::debug("language_server_command: using Razor language server");
                let rzls_path = self.get_razor_path(language_server_id, worktree)?;

                Ok(zed::Command {
                    command: rzls_path,
                    args: vec![],
                    env: Default::default(),
                })
            }
            _ => {
                // Default to Roslyn for any other C# related language server ID
                logger::Logger::debug("language_server_command: using Roslyn language server");
                let roslyn_path = self.get_roslyn_path(language_server_id, worktree)?;

                let binary_settings = LspSettings::for_worktree("csharp-roslyn", worktree)
                    .ok()
                    .and_then(|lsp_settings| lsp_settings.binary);
                let mut binary_args = binary_settings
                    .as_ref()
                    .and_then(|binary_settings| binary_settings.arguments.clone())
                    .unwrap_or_default();

                // Add required Roslyn arguments
                binary_args.push("--logLevel".to_string());
                binary_args.push("Information".to_string());

                // Set extension log directory to the extension folder for easy access
                binary_args.push("--extensionLogDirectory".to_string());
                binary_args.push("./logs".to_string());

                logger::Logger::debug(&format!(
                    "language_server_command: using Roslyn at: {}",
                    roslyn_path
                ));
                logger::Logger::debug(&format!(
                    "language_server_command: Roslyn args: {:?}",
                    binary_args
                ));

                // Extract the directory containing the Roslyn binary
                let roslyn_dir = std::path::Path::new(&roslyn_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or(".")
                    .to_string();

                logger::Logger::debug(&format!(
                    "language_server_command: Roslyn directory: {}",
                    roslyn_dir
                ));

                Ok(zed::Command {
                    command: roslyn_path.clone(),
                    args: binary_args,
                    env: Default::default(),
                })
            }
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let server_id_str = format!("{:?}", language_server_id);
        logger::Logger::debug(&format!(
            "language_server_workspace_configuration: server_id = {}",
            server_id_str
        ));

        if server_id_str.contains("rzls") {
            // Razor-specific configuration
            logger::Logger::debug(
                "language_server_workspace_configuration: using Razor configuration",
            );
            Ok(Some(serde_json::json!({})))
        } else {
            // Roslyn configuration with inlay hints
            logger::Logger::debug("language_server_workspace_configuration: using Roslyn configuration with inlay hints");
            // Provide both the nested `csharp.inlayHints` shape and the flat `dotnet.inlayHints.*` keys
            // so that servers expecting either configuration shape will receive the settings.
            let config = serde_json::json!({
                "csharp": {
                    "inlayHints": {
                        "parameters": {
                            "enabled": true,
                            "forLiteralParameters": true,
                            "forIndexerParameters": true,
                            "forObjectCreationParameters": true,
                            "forOtherParameters": true,
                            "suppressForParametersThatDifferOnlyBySuffix": false,
                            "suppressForParametersThatMatchMethodIntent": false,
                            "suppressForParametersThatMatchArgumentName": false
                        },
                        "types": {
                            "enabled": true,
                            "forImplicitVariableTypes": true,
                            "forLambdaParameterTypes": true,
                            "forImplicitObjectCreation": true
                        }
                    }
                },
                "dotnet": {
                    "inlayHints": {
                        "enableInlayHintsForParameters": true,
                        "enableInlayHintsForLiteralParameters": true,
                        "enableInlayHintsForIndexerParameters": true,
                        "enableInlayHintsForObjectCreationParameters": true,
                        "enableInlayHintsForOtherParameters": true,
                        "suppressInlayHintsForParametersThatDifferOnlyBySuffix": false,
                        "suppressInlayHintsForParametersThatMatchMethodIntent": false,
                        "suppressInlayHintsForParametersThatMatchArgumentName": false,
                        "enableInlayHintsForTypes": true,
                        "enableInlayHintsForImplicitVariableTypes": true,
                        "enableInlayHintsForLambdaParameterTypes": true,
                        "enableInlayHintsForImplicitObjectCreation": true
                    }
                }
            });

            Ok(Some(config))
        }
    }
}

zed::register_extension!(CsharpExtension);
