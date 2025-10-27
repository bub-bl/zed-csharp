use crate::logger;
use std::fs;
use std::path::Path;

/// Builds Roslyn language server commands with proper arguments
pub struct RoslynCommandBuilder {
    binary_path: String,
    log_level: String,
    extension_log_dir: String,
}

impl RoslynCommandBuilder {
    pub fn new(binary_path: String, extension_log_dir: String) -> Self {
        Self {
            binary_path,
            log_level: "Information".to_string(),
            extension_log_dir,
        }
    }

    /// Set the log level (Debug, Information, Warning, Error)
    pub fn with_log_level(mut self, level: &str) -> Self {
        self.log_level = level.to_string();
        self
    }

    /// Build command for C# only (Roslyn without Razor)
    pub fn build_csharp_command(self) -> (String, Vec<String>) {
        logger::Logger::debug("RoslynCommandBuilder: building C# command");

        // Ensure the log directory exists
        if !self.extension_log_dir.is_empty() {
            if let Err(e) = fs::create_dir_all(&self.extension_log_dir) {
                logger::Logger::warn(&format!(
                    "RoslynCommandBuilder: failed to create log directory {}: {}",
                    self.extension_log_dir, e
                ));
            }
        }

        let args = vec![
            "--logLevel".to_string(),
            self.log_level,
            "--extensionLogDirectory".to_string(),
            self.extension_log_dir,
        ];

        logger::Logger::debug(&format!("RoslynCommandBuilder: C# args: {:?}", args));

        (self.binary_path, args)
    }

    /// Build command for Razor support (Roslyn with Razor extensions)
    pub fn build_razor_command(
        self,
        razor_compiler_dll: Option<String>,
        razor_targets_path: Option<String>,
        razor_extension_dll: Option<String>,
    ) -> (String, Vec<String>) {
        logger::Logger::debug("RoslynCommandBuilder: building Razor command");

        // Ensure the log directory exists
        if !self.extension_log_dir.is_empty() {
            if let Err(e) = fs::create_dir_all(&self.extension_log_dir) {
                logger::Logger::warn(&format!(
                    "RoslynCommandBuilder: failed to create log directory {}: {}",
                    self.extension_log_dir, e
                ));
            }
        }

        let mut args = vec![
            "--logLevel".to_string(),
            self.log_level,
            "--extensionLogDirectory".to_string(),
            self.extension_log_dir,
        ];

        // Add Razor compiler DLL if provided
        if let Some(compiler_dll) = razor_compiler_dll {
            logger::Logger::debug(&format!(
                "RoslynCommandBuilder: adding Razor compiler: {}",
                compiler_dll
            ));
            args.push("--razorSourceGenerator".to_string());
            args.push(compiler_dll);
        }

        // Add Razor design-time targets if provided
        if let Some(targets_path) = razor_targets_path {
            logger::Logger::debug(&format!(
                "RoslynCommandBuilder: adding Razor targets: {}",
                targets_path
            ));
            args.push("--razorDesignTimePath".to_string());
            args.push(targets_path);
        }

        // Add Razor extension DLL if provided
        if let Some(extension_dll) = razor_extension_dll {
            logger::Logger::debug(&format!(
                "RoslynCommandBuilder: adding Razor extension: {}",
                extension_dll
            ));
            args.push("--extension".to_string());
            args.push(extension_dll);
        }

        logger::Logger::debug(&format!("RoslynCommandBuilder: Razor args: {:?}", args));

        (self.binary_path, args)
    }
}

/// Discovers Razor-related DLLs and paths in a VS Code C# extension installation
pub struct RazorSupport {
    version_dir: String,
}

impl RazorSupport {
    pub fn new(version_dir: String) -> Self {
        Self { version_dir }
    }

    /// Find Razor compiler DLL
    pub fn find_razor_compiler_dll(&self) -> Option<String> {
        let possible_paths = vec![
            format!(
                "{}/extension/.razor/Microsoft.CodeAnalysis.Razor.Compiler.dll",
                self.version_dir
            ),
            format!(
                "{}/extension/Microsoft.CodeAnalysis.Razor.Compiler.dll",
                self.version_dir
            ),
        ];

        for path in possible_paths {
            if Path::new(&path).exists() {
                logger::Logger::debug(&format!("RazorSupport: found Razor compiler at: {}", path));
                return Some(path);
            }
        }

        logger::Logger::warn("RazorSupport: Razor compiler DLL not found");
        None
    }

    /// Find Razor design-time targets
    pub fn find_razor_targets(&self) -> Option<String> {
        let possible_paths = vec![
            format!(
                "{}/extension/.razor/Targets/Microsoft.NET.Sdk.Razor.DesignTime.targets",
                self.version_dir
            ),
            format!(
                "{}/extension/Targets/Microsoft.NET.Sdk.Razor.DesignTime.targets",
                self.version_dir
            ),
        ];

        for path in possible_paths {
            if Path::new(&path).exists() {
                logger::Logger::debug(&format!("RazorSupport: found Razor targets at: {}", path));
                return Some(path);
            }
        }

        logger::Logger::warn("RazorSupport: Razor targets not found");
        None
    }

    /// Find Razor extension DLL
    pub fn find_razor_extension_dll(&self) -> Option<String> {
        let possible_paths = vec![
            format!(
                "{}/extension/.razor/RazorExtension/Microsoft.VisualStudioCode.RazorExtension.dll",
                self.version_dir
            ),
            format!(
                "{}/extension/RazorExtension/Microsoft.VisualStudioCode.RazorExtension.dll",
                self.version_dir
            ),
        ];

        for path in possible_paths {
            if Path::new(&path).exists() {
                logger::Logger::debug(&format!("RazorSupport: found Razor extension at: {}", path));
                return Some(path);
            }
        }

        logger::Logger::warn("RazorSupport: Razor extension DLL not found");
        None
    }

    /// Check if all Razor components are available
    pub fn is_razor_available(&self) -> bool {
        let compiler = self.find_razor_compiler_dll().is_some();
        let targets = self.find_razor_targets().is_some();
        let extension = self.find_razor_extension_dll().is_some();

        let available = compiler && targets && extension;
        logger::Logger::debug(&format!(
            "RazorSupport: Razor components available - compiler: {}, targets: {}, extension: {}, total: {}",
            compiler, targets, extension, available
        ));
        available
    }

    /// Get all Razor components if available
    pub fn get_razor_components(&self) -> Option<RazorComponents> {
        if self.is_razor_available() {
            Some(RazorComponents {
                compiler_dll: self.find_razor_compiler_dll().unwrap(),
                targets_path: self.find_razor_targets().unwrap(),
                extension_dll: self.find_razor_extension_dll().unwrap(),
            })
        } else {
            None
        }
    }
}

/// Container for Razor component paths
#[derive(Debug, Clone)]
pub struct RazorComponents {
    pub compiler_dll: String,
    pub targets_path: String,
    pub extension_dll: String,
}
