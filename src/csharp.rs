use std::fs;
use zed_extension_api::{self as zed, serde_json, settings::LspSettings, LanguageServerId, Result};

struct CsharpExtension {
    cached_binary_path: Option<String>,
}

impl CsharpExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        let binary_settings = LspSettings::for_worktree("csharp-language-server", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);

        // Check if user specified a custom path
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path) {
            return Ok(path);
        }

        // Check if csharp-ls is available in PATH
        if let Some(path) = worktree.which("csharp-ls") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // Check cached binary
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        // Check if dotnet is available
        if worktree.which("dotnet").is_none() {
            return Err(concat!(
                ".NET SDK not found.\n\n",
                "Please install .NET SDK from: https://dotnet.microsoft.com/download\n",
                "After installation, restart Zed."
            ).to_string());
        }

        // Try to install csharp-ls
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        // Install to a local tools directory within the extension workspace
        let tools_dir = "csharp-tools";
        fs::create_dir_all(&tools_dir)
            .map_err(|e| format!("failed to create tools directory: {e}"))?;

        // We can't directly execute commands, but we can try using the download functionality
        // As a fallback, provide clear instructions
        Err(concat!(
            "csharp-ls not found and automatic installation is not supported.\n\n",
            "To install it, run:\n",
            "  dotnet tool install --global csharp-ls\n\n",
            "After installation, restart Zed."
        ).to_string())
    }
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree("csharp-language-server", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.binary);
        let binary_args = binary_settings
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        let server_path = self.language_server_binary(language_server_id, worktree)?;

        Ok(zed::Command {
            command: server_path,
            args: binary_args.unwrap_or_default(),
            env: Default::default(),
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let user_config = LspSettings::for_worktree("csharp-language-server", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone());

        let config = match user_config {
            Some(user) => user,
            None => {
                // Default configuration for csharp-ls with inlay hints enabled
                serde_json::json!({
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
                })
            }
        };

        Ok(Some(config))
    }
}

zed::register_extension!(CsharpExtension);