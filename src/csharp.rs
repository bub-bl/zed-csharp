mod binary_manager;
mod command_builder;
mod extension_config;
mod logger;
mod path_utils;
mod version_config;

// Language server identifiers
const DEBUG_ADAPTER_NETCOREDBG: &str = "netcoredbg";
const LSP_CSHARP_ROSLYN: &str = "csharp-roslyn";
const LSP_CSHARP_RAZOR: &str = "csharp-razor";
const LSP_RZLS_ID: &str = "rzls";

use binary_manager::BinaryManager;
use command_builder::RazorSupport;
use extension_config::ExtensionConfig;
use std::fs;
use version_config::{netcoredbg_config, vscode_csharp_config};
use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    settings::LspSettings,
    DebugAdapterBinary, DebugTaskDefinition, LanguageServerId, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

struct CsharpExtension {
    binary_manager: BinaryManager,
    config: ExtensionConfig,
    cached_roslyn_path: Option<String>,
    cached_razor_path: Option<String>,
    cached_debugger_path: Option<String>,
}

impl CsharpExtension {
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

        let binary_settings = LspSettings::for_worktree(LSP_CSHARP_ROSLYN, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            logger::Logger::debug(&format!(
                "get_roslyn_path: using user-defined path: {}",
                path
            ));
            let absolute_path = path_utils::normalize_path_to_absolute(&path);
            return Ok(absolute_path);
        }

        // check for cached roslyn path
        if let Some(path) = &self.cached_roslyn_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_roslyn_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_roslyn_path: resolving version directory");
        let version_dir = self
            .binary_manager
            .get_version_dir(&vscode_csharp_config(), Some(language_server_id))?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
            _ => "Microsoft.CodeAnalysis.LanguageServer",
        };

        let roslyn_path = path_utils::normalize_path_to_absolute(&format!("{}/extension/.roslyn/{}", version_dir, binary_name));
        logger::Logger::debug(&format!("get_roslyn_path: resolved path {}", roslyn_path));

        if !fs::metadata(&roslyn_path).map_or(false, |stat| stat.is_file()) {
            logger::Logger::debug("get_roslyn_path: failed to find roslyn file");
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

        // version_dir is already absolute, so roslyn_path is absolute too
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

        let binary_settings = LspSettings::for_worktree(LSP_CSHARP_RAZOR, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check for user-defined path first
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            logger::Logger::debug(&format!(
                "get_razor_path: using user-defined path: {}",
                path
            ));
            let absolute_path = path_utils::normalize_path_to_absolute(&path);
            return Ok(absolute_path);
        }

        // check for cached razor path
        if let Some(path) = &self.cached_razor_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                logger::Logger::debug(&format!("get_razor_path: using cached path: {}", path));
                return Ok(path.clone());
            }
        }

        logger::Logger::debug("get_razor_path: resolving version directory");
        let version_dir = self
            .binary_manager
            .get_version_dir(&vscode_csharp_config(), Some(language_server_id))?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "rzls.exe",
            _ => "rzls",
        };

        let razor_path = path_utils::normalize_path_to_absolute(&format!("{}/extension/.razor/{}", version_dir, binary_name));
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

        // version_dir is already absolute, so razor_path is absolute too
        logger::Logger::debug(&format!("get_razor_path: found at {}", razor_path));
        self.cached_razor_path = Some(razor_path.clone());
        Ok(razor_path)
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
        let version_dir = self
            .binary_manager
            .get_version_dir(&netcoredbg_config(), None)?;

        let (platform, _) = zed::current_platform();
        let binary_name = match platform {
            zed::Os::Windows => "netcoredbg.exe",
            _ => "netcoredbg",
        };

        let debugger_path = format!("{}/{}", version_dir, binary_name);

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

    /// Build Roslyn command with proper arguments
    fn build_roslyn_command(
        &self,
        roslyn_path: &str,
        version_dir: &str,
        worktree: &zed::Worktree,
    ) -> (String, Vec<String>) {
        use command_builder::RoslynCommandBuilder;

        logger::Logger::debug("build_roslyn_command: building Roslyn command");

        let binary_settings = LspSettings::for_worktree(LSP_CSHARP_ROSLYN, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone())
            .unwrap_or_default();

        let logs_dir = format!("{}/logs", version_dir);
        let builder = RoslynCommandBuilder::new(roslyn_path.to_string(), logs_dir)
            .with_log_level(&self.config.log_level);
        let (cmd, mut args) = builder.build_csharp_command();

        // Add any user-provided arguments
        args.extend(binary_args);

        logger::Logger::debug(&format!("build_roslyn_command: final args: {:?}", args));

        (cmd, args)
    }

    /// Build Roslyn command with Razor support if available
    fn build_roslyn_razor_command(
        &self,
        roslyn_path: &str,
        version_dir: &str,
        worktree: &zed::Worktree,
    ) -> (String, Vec<String>) {
        use command_builder::RoslynCommandBuilder;

        logger::Logger::debug("build_roslyn_razor_command: building Roslyn with Razor support");

        let razor_support = RazorSupport::new(version_dir.to_string());

        let binary_settings = LspSettings::for_worktree(LSP_CSHARP_ROSLYN, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone())
            .unwrap_or_default();

        let logs_dir = format!("{}/logs", version_dir);
        let builder = RoslynCommandBuilder::new(roslyn_path.to_string(), logs_dir)
            .with_log_level(&self.config.log_level);

        let (cmd, mut args) = if let Some(razor_components) = razor_support.get_razor_components() {
            logger::Logger::debug(
                "build_roslyn_razor_command: Razor components available, enabling Razor support",
            );
            builder.build_razor_command(
                Some(razor_components.compiler_dll),
                Some(razor_components.targets_path),
                Some(razor_components.extension_dll),
            )
        } else {
            logger::Logger::debug(
                "build_roslyn_razor_command: Razor components not available, using C# only",
            );
            builder.build_csharp_command()
        };

        // Add any user-provided arguments
        args.extend(binary_args);

        logger::Logger::debug(&format!(
            "build_roslyn_razor_command: final args: {:?}",
            args
        ));

        (cmd, args)
    }

    /// Check if working with Razor files
    fn has_razor_files(&self, worktree: &zed::Worktree) -> bool {
        let root_path = worktree.root_path();

        if let Ok(entries) = fs::read_dir(&root_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext == "razor" || ext == "cshtml" {
                            logger::Logger::debug(&format!(
                                "has_razor_files: found Razor file: {}",
                                path.display()
                            ));
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            binary_manager: BinaryManager::new(),
            config: ExtensionConfig::default(),
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

        match server_id_str.as_str() {
            s if s.contains(LSP_RZLS_ID) => {
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
                let version_dir = self
                    .binary_manager
                    .get_version_dir(&vscode_csharp_config(), Some(language_server_id))?;

                // Detect if workspace has Razor files
                let has_razor = self.has_razor_files(worktree);
                logger::Logger::debug(&format!(
                    "language_server_command: workspace has Razor files: {}",
                    has_razor
                ));

                // Build appropriate command based on Razor availability
                let (cmd, args) = if has_razor {
                    logger::Logger::debug(
                        "language_server_command: building Roslyn command with Razor support",
                    );
                    self.build_roslyn_razor_command(&roslyn_path, &version_dir, worktree)
                } else {
                    logger::Logger::debug(
                        "language_server_command: building C#-only Roslyn command",
                    );
                    self.build_roslyn_command(&roslyn_path, &version_dir, worktree)
                };

                logger::Logger::debug(&format!(
                    "language_server_command: using Roslyn at: {}",
                    roslyn_path
                ));
                logger::Logger::debug(&format!("language_server_command: Roslyn args: {:?}", args));

                Ok(zed::Command {
                    command: cmd,
                    args,
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

        if server_id_str.contains(LSP_RZLS_ID) {
            // Razor-specific configuration
            logger::Logger::debug(
                "language_server_workspace_configuration: using Razor configuration",
            );
            Ok(Some(serde_json::json!({})))
        } else {
            // Roslyn LSP configuration with comprehensive settings
            logger::Logger::debug("language_server_workspace_configuration: using Roslyn LSP configuration");
            // Roslyn LSP uses pipe-delimited keys for configuration sections
            let config = serde_json::json!({
                "csharp|inlay_hints": {
                    "dotnet_enable_inlay_hints_for_parameters": true,
                    "dotnet_enable_inlay_hints_for_literal_parameters": true,
                    "dotnet_enable_inlay_hints_for_indexer_parameters": true,
                    "dotnet_enable_inlay_hints_for_object_creation_parameters": true,
                    "dotnet_enable_inlay_hints_for_other_parameters": true,
                    "dotnet_suppress_inlay_hints_for_parameters_that_differ_only_by_suffix": false,
                    "dotnet_suppress_inlay_hints_for_parameters_that_match_method_intent": false,
                    "dotnet_suppress_inlay_hints_for_parameters_that_match_argument_name": false,
                    "csharp_enable_inlay_hints_for_types": true,
                    "csharp_enable_inlay_hints_for_implicit_variable_types": true,
                    "csharp_enable_inlay_hints_for_lambda_parameter_types": true,
                    "csharp_enable_inlay_hints_for_implicit_object_creation": true
                },
                "csharp|background_analysis": {
                    "dotnet_analyzer_diagnostics_scope": "fullSolution",
                    "dotnet_compiler_diagnostics_scope": "fullSolution"
                },
                "csharp|code_lens": {
                    "dotnet_enable_references_code_lens": true,
                    "dotnet_enable_tests_code_lens": true
                },
                "csharp|completion": {
                    "dotnet_provide_regex_completions": false,
                    "dotnet_show_completion_items_from_unimported_namespaces": true,
                    "dotnet_show_name_completion_suggestions": true
                },
                "csharp|symbol_search": {
                    "dotnet_search_reference_assemblies": true
                },
                "csharp|formatting": {
                    "dotnet_organize_imports_on_format": true
                }
            });

            Ok(Some(config))
        }
    }
}

zed::register_extension!(CsharpExtension);
