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
        // First check if we can get version from cached paths
        if let Some(path) = &self.cached_roslyn_path {
            if let Some(dir) = path.split('/').next() {
                if fs::metadata(dir).map_or(false, |stat| stat.is_dir()) {
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

        // If we already have this version locally, use it
        if fs::metadata(&version_dir).map_or(false, |stat| stat.is_dir()) {
            if let Some(language_server_id) = language_server_id {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
            }
            return Ok(version_dir);
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

        let vsix_url = format!(
            "https://ms-dotnettools.gallery.vsassets.io/_apis/public/gallery/publisher/ms-dotnettools/extension/csharp/{}/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage?redirect=true&targetPlatform={}",
            version, platform_str
        );

        zed::download_file(&vsix_url, &version_dir, zed::DownloadedFileType::Zip)
            .map_err(|e| format!("failed to download VS Code C# extension: {e}"))?;

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
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        let binary_settings = LspSettings::for_worktree("csharp-roslyn", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            return Ok(path);
        }

        // check for cached roslyn path
        if let Some(path) = &self.cached_roslyn_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

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
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        let binary_settings = LspSettings::for_worktree("csharp-razor", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            return Ok(path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        let version_dir = self.get_version_dir(Some(language_server_id))?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "rzls.exe",
            _ => "rzls",
        };

        let razor_path = format!("{}/extension/.razor/{}", version_dir, binary_name);

        if !fs::metadata(&razor_path).map_or(false, |stat| stat.is_file()) {
            return Err(format!(
                "Razor language server not found at: {}",
                razor_path
            ));
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

        self.cached_razor_path = Some(razor_path.clone());
        Ok(razor_path)
    }

    fn get_debugger_path(&mut self, user_provided_path: Option<String>) -> Result<String, String> {
        // check for user-defined path first
        if let Some(user_path) = user_provided_path {
            return Ok(user_path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

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
        if adapter_name != "coreclr" {
            return Err(format!("Cannot create binary for adapter: {adapter_name}"));
        }

        let configuration = config.config.to_string();

        let debugger_path = self
            .get_debugger_path(user_provided_debug_adapter_path)
            .map_err(|e| format!("Failed to locate C# debugger: {}", e))?;

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

        match server_id_str.as_str() {
            s if s.contains("rzls") => {
                // Razor Language Server
                let rzls_path = self.get_razor_path(language_server_id, worktree)?;

                Ok(zed::Command {
                    command: rzls_path,
                    args: vec![],
                    env: Default::default(),
                })
            }
            _ => {
                // Default to Roslyn for any other C# related language server ID
                let roslyn_path = self.get_roslyn_path(language_server_id, worktree)?;

                let binary_settings = LspSettings::for_worktree("roslyn", worktree)
                    .ok()
                    .and_then(|lsp_settings| lsp_settings.binary);
                let binary_args = binary_settings
                    .as_ref()
                    .and_then(|binary_settings| binary_settings.arguments.clone());

                Ok(zed::Command {
                    command: roslyn_path,
                    args: binary_args.unwrap_or_default(),
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

        if server_id_str.contains("rzls") {
            // Razor-specific configuration
            Ok(Some(serde_json::json!({})))
        } else {
            // Roslyn configuration with inlay hints
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
