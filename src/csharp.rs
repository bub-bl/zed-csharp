mod logger;

use std::fs;
use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    settings::LspSettings,
    DebugAdapterBinary, DebugTaskDefinition, LanguageServerId, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

struct CsharpExtension {
    cached_roslyn_path: Option<String>,
    cached_razor_path: Option<String>,
    cached_debugger_path: Option<String>,
}

impl CsharpExtension {
    fn get_version_dir(&mut self, language_server_id: Option<&LanguageServerId>) -> Result<String> {
        logger::Logger::debug("get_version_dir: starting version check");
        
        // First check if we can get version from cached paths
        if let Some(path) = &self.cached_roslyn_path {
            if let Some(dir) = path.split('/').next() {
                if fs::metadata(dir).map_or(false, |stat| stat.is_dir()) {
                    logger::Logger::debug(&format!("get_version_dir: using cached directory: {}", dir));
                    return Ok(dir.to_string());
                }
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
                logger::Logger::debug(&format!("get_version_dir: validated existing directory: {}", version_dir));
                if let Some(language_server_id) = language_server_id {
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                }
                return Ok(version_dir);
            } else {
                // Directory exists but is incomplete/corrupted, clean it up
                logger::Logger::warn(&format!("get_version_dir: found incomplete directory, removing: {}", version_dir));
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
        logger::Logger::debug(&format!("get_version_dir: created directory: {}", version_dir));

        let vsix_url = format!(
            "https://ms-dotnettools.gallery.vsassets.io/_apis/public/gallery/publisher/ms-dotnettools/extension/csharp/{}/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage?redirect=true&targetPlatform={}",
            version, platform_str
        );

        logger::Logger::debug(&format!("get_version_dir: downloading from {}", vsix_url));

        // Try to download and extract
        let download_result = zed::download_file(&vsix_url, &version_dir, zed::DownloadedFileType::Zip);
        
        // Don't trust the result - check if the folder actually has content
        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };
        let roslyn_binary = format!("{}/extension/.roslyn/{}", version_dir, binary_name);
        let has_content = fs::metadata(&roslyn_binary).map_or(false, |stat| stat.is_file());

        match (download_result, has_content) {
            (Ok(_), true) => {
                logger::Logger::debug(&format!("get_version_dir: successfully downloaded and extracted to {}", version_dir));
            }
            (Ok(_), false) => {
                logger::Logger::warn(&format!("get_version_dir: download reported success but missing Roslyn binary at {}", roslyn_binary));
                logger::Logger::debug(&format!("get_version_dir: proceeding anyway since extraction succeeded"));
            }
            (Err(e), true) => {
                logger::Logger::warn(&format!("get_version_dir: download reported error but files were extracted: {}", e));
                logger::Logger::debug(&format!("get_version_dir: proceeding since Roslyn binary exists at {}", roslyn_binary));
            }
            (Err(e), false) => {
                logger::Logger::error(&format!("get_version_dir: download failed and no content found: {}", e));
                // Clean up the corrupted directory
                logger::Logger::debug(&format!("get_version_dir: cleaning up corrupted directory: {}", version_dir));
                fs::remove_dir_all(&version_dir).ok();
                return Err(format!("failed to download VS Code C# extension: {e}. Cleaned up partial download."));
            }
        }

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
            logger::Logger::debug(&format!("get_roslyn_path: using user-defined path: {}", path));
            return Ok(path);
        }

        // check for cached roslyn path
        if let Some(path) = &self.cached_roslyn_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_roslyn_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_roslyn_path: downloading extension");
        let version_dir = self.get_version_dir(Some(language_server_id))?;

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

        self.cached_roslyn_path = Some(roslyn_path.clone());
        Ok(roslyn_path)
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
            logger::Logger::debug(&format!("get_razor_path: using user-defined path: {}", path));
            return Ok(path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_razor_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_razor_path: resolving version directory for download");
        let version_dir = self.get_version_dir(Some(language_server_id))?;

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

        logger::Logger::debug(&format!("get_razor_path: found at {}", razor_path));
        self.cached_razor_path = Some(razor_path.clone());
        Ok(razor_path)
    }

    fn get_debugger_path(&mut self, user_provided_path: Option<String>) -> Result<String, String> {
        logger::Logger::debug("get_debugger_path: starting debugger path resolution");
        
        // check for user-defined path first
        if let Some(user_path) = user_provided_path {
            logger::Logger::debug(&format!("get_debugger_path: using user-provided path: {}", user_path));
            return Ok(user_path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_debugger_path: using cached debugger path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_debugger_path: getting version directory");
        let version_dir = self.get_version_dir(None)?;

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

        self.cached_debugger_path = Some(debugger_path.clone());
        Ok(debugger_path)
    }
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            cached_roslyn_path: None,
            cached_razor_path: None,
            cached_debugger_path: None,
        }
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        logger::Logger::debug(&format!("get_dap_binary: requested adapter: {}", adapter_name));
        
        if adapter_name != "coreclr" {
            logger::Logger::error(&format!("get_dap_binary: unsupported adapter: {}", adapter_name));
            return Err(format!("Cannot create binary for adapter: {adapter_name}"));
        }

        let configuration = config.config.to_string();

        let debugger_path = self
            .get_debugger_path(user_provided_debug_adapter_path)
            .map_err(|e| {
                logger::Logger::error(&format!("get_dap_binary: failed to locate debugger: {}", e));
                format!("Failed to locate C# debugger: {}", e)
            })?;

        logger::Logger::debug(&format!("get_dap_binary: using debugger at: {}", debugger_path));

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
        logger::Logger::debug(&format!("language_server_command: server_id: {}", server_id_str));

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

                let binary_settings = LspSettings::for_worktree("roslyn", worktree)
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

                logger::Logger::debug(&format!("language_server_command: using Roslyn at: {}", roslyn_path));
                logger::Logger::debug(&format!("language_server_command: Roslyn args: {:?}", binary_args));

                // Extract the directory containing the Roslyn binary
                let roslyn_dir = std::path::Path::new(&roslyn_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or(".")
                    .to_string();
                
                logger::Logger::debug(&format!("language_server_command: Roslyn directory: {}", roslyn_dir));

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
        logger::Logger::debug(&format!("language_server_workspace_configuration: server_id = {}", server_id_str));

        if server_id_str.contains("rzls") {
            // Razor-specific configuration
            logger::Logger::debug("language_server_workspace_configuration: using Razor configuration");
            Ok(Some(serde_json::json!({})))
        } else {
            // Roslyn configuration with inlay hints
            logger::Logger::debug("language_server_workspace_configuration: using Roslyn configuration with inlay hints");
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
                }
            });

            Ok(Some(config))
        }
    }
}

zed::register_extension!(CsharpExtension);
