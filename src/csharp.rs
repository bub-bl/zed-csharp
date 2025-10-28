mod binary_manager;
mod logger;
mod path_utils;
mod version_config;

// Language server identifiers
const DEBUG_ADAPTER_NETCOREDBG: &str = "netcoredbg";
const LANGUAGE_SERVER_NAME: &str = "csharp-language-server";

use binary_manager::BinaryManager;
use std::fs;
use version_config::{csharp_language_server_config, netcoredbg_config,};
use zed_extension_api::{
    self as zed,
    serde_json::{Value},
    settings::LspSettings,
    DebugAdapterBinary, DebugTaskDefinition, LanguageServerId, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

struct CsharpExtension {
    binary_manager: BinaryManager,
    cached_debugger_path: Option<String>,
    cached_language_server_path: Option<String>,
    platform_os: zed::Os,
    _platform_arch: zed::Architecture,
}

impl CsharpExtension {
    fn get_language_server_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        logger::Logger::debug(&format!(
            "get_language_server_path: starting {} path resolution",
            LANGUAGE_SERVER_NAME
        ));

        let binary_settings = LspSettings::for_worktree(LANGUAGE_SERVER_NAME, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            logger::Logger::debug(&format!(
                "get_language_server_path: using user-defined path: {}",
                path
            ));
            let absolute_path = path_utils::normalize_path_to_absolute(&path);
            return Ok(absolute_path);
        }

        // Check for cached path
        if let Some(path) = &self.cached_language_server_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!(
                    "get_language_server_path: using cached path: {}",
                    path
                ));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_language_server_path: resolving version directory");

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let config = csharp_language_server_config();
        let version_dir = self
            .binary_manager
            .get_version_dir(&config, Some(language_server_id))?;

        let server_path = (config.get_binary_path)(&version_dir);
        logger::Logger::debug(&format!(
            "get_language_server_path: resolved path {}",
            server_path
        ));

        if !fs::metadata(&server_path).map_or(false, |stat| stat.is_file()) {
            logger::Logger::debug("get_language_server_path: failed to find binary");
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(format!(
                    "{} not found at: {}",
                    LANGUAGE_SERVER_NAME, server_path
                )),
            );
            return Err(format!(
                "{} binary not found at: {}",
                LANGUAGE_SERVER_NAME, server_path
            ));
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&server_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&server_path, perms).ok();
            }
        }

        // Cache the path before returning
        self.cached_language_server_path = Some(server_path.clone());

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );
        logger::Logger::debug(&format!(
            "get_language_server_path: found and cached at {}",
            server_path
        ));
        Ok(server_path)
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

        // check for cached debugger path
        if let Some(path) = &self.cached_debugger_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!(
                    "get_debugger_path: using cached debugger path: {}",
                    path
                ));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_debugger_path: getting version directory");
        let config = netcoredbg_config();
        let version_dir = self
            .binary_manager
            .get_version_dir(&config, None)?;

        let debugger_path = (config.get_binary_path)(&version_dir);

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

        // version_dir is already absolute, so debugger_path is absolute too
        self.cached_debugger_path = Some(debugger_path.clone());
        Ok(debugger_path)
    }
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        let (platform_os, platform_arch) = zed::current_platform();
        Self {
            binary_manager: BinaryManager::new(),
            cached_debugger_path: None,
            cached_language_server_path: None,
            platform_os: platform_os,
            _platform_arch: platform_arch,
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

        // Accept "coreclr" as alternative name, but always use netcoredbg
        if adapter_name != DEBUG_ADAPTER_NETCOREDBG && adapter_name != "coreclr" {
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
        if adapter_name != DEBUG_ADAPTER_NETCOREDBG && adapter_name != "coreclr" {
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

        let dotnet_path = worktree.which("dotnet").ok_or_else(|| {
            "dotnet runtime not found. Please ensure .NET is installed and in your PATH.".to_string()
        })?;

        logger::Logger::debug(&format!(
            "language_server_command: using dotnet at: {}",
            dotnet_path
        ));

        let server_path = self.get_language_server_path(language_server_id, worktree)?;

        logger::Logger::debug(&format!(
            "language_server_command: using {} at: {}",
            LANGUAGE_SERVER_NAME, server_path
        ));

        Ok(zed::Command {
            command: dotnet_path,
            args: vec![server_path],
            env: Default::default(),
        })
    }
}

zed::register_extension!(CsharpExtension);
