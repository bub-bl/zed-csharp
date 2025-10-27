/// Configuration for version directory download
pub struct VersionDirConfig {
    /// Directory prefix (e.g., "vscode-csharp" or "netcoredbg")
    pub prefix: String,
    /// GitHub repository (e.g., "dotnet/vscode-csharp")
    pub github_repo: String,
    /// Function to get the binary path relative to version_dir
    pub get_binary_path: fn(&str) -> String,
    /// Binary name for logging
    pub binary_name_for_logging: String,
    /// Function to resolve download URL - fetches from GitHub releases or uses fallback
    /// Returns download_url given (version, platform)
    pub get_download_url: fn(&str, &str) -> Result<String, String>,
    /// Function to get the platform string for this package
    /// Different packages use different naming conventions (darwin vs osx, win32 vs win, etc.)
    pub get_platform_string: fn() -> Result<String, String>,
}

/// Builder for creating version configs
pub struct VersionConfigBuilder {
    prefix: String,
    github_repo: String,
    get_binary_path: fn(&str) -> String,
    binary_name_for_logging: String,
    get_download_url: fn(&str, &str) -> Result<String, String>,
    get_platform_string: fn() -> Result<String, String>,
}

impl VersionConfigBuilder {
    pub fn new(prefix: &str, github_repo: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            github_repo: github_repo.to_string(),
            get_binary_path: |_| String::new(),
            binary_name_for_logging: String::new(),
            get_download_url: |_, _| Err("not configured".to_string()),
            get_platform_string: || Err("not configured".to_string()),
        }
    }

    pub fn get_binary_path(mut self, builder: fn(&str) -> String) -> Self {
        self.get_binary_path = builder;
        self
    }

    pub fn binary_name_for_logging(mut self, name: &str) -> Self {
        self.binary_name_for_logging = name.to_string();
        self
    }

    pub fn get_download_url(mut self, resolver: fn(&str, &str) -> Result<String, String>) -> Self {
        self.get_download_url = resolver;
        self
    }

    pub fn get_platform_string(mut self, resolver: fn() -> Result<String, String>) -> Self {
        self.get_platform_string = resolver;
        self
    }

    pub fn build(self) -> VersionDirConfig {
        VersionDirConfig {
            prefix: self.prefix,
            github_repo: self.github_repo,
            get_binary_path: self.get_binary_path,
            binary_name_for_logging: self.binary_name_for_logging,
            get_download_url: self.get_download_url,
            get_platform_string: self.get_platform_string,
        }
    }
}

/// Create a configuration for netcoredbg
pub fn netcoredbg_config() -> VersionDirConfig {
    VersionConfigBuilder::new("netcoredbg", "marcptrs/netcoredbg")
        .get_platform_string(|| {
            use zed_extension_api as zed;
            let (platform, arch) = zed::current_platform();
            let platform_str = match (platform, arch) {
                (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
                (zed::Os::Linux, zed::Architecture::X86) => "linux-x86",
                (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64",
                (zed::Os::Mac, zed::Architecture::Aarch64) => "osx-arm64",
                (zed::Os::Mac, zed::Architecture::X86) => "osx-x86",
                (zed::Os::Mac, zed::Architecture::X8664) => "osx-x64",
                (zed::Os::Windows, zed::Architecture::Aarch64) => "win-arm64",
                (zed::Os::Windows, zed::Architecture::X86) => "win-x86",
                (zed::Os::Windows, zed::Architecture::X8664) => "win-x64",
            };
            Ok(platform_str.to_string())
        })
        .get_download_url(|_version: &str, platform: &str| {
            use zed_extension_api as zed;

            // Fetch the latest release from GitHub
            let release = zed::latest_github_release(
                "marcptrs/netcoredbg",
                zed::GithubReleaseOptions {
                    require_assets: true,
                    pre_release: false,
                },
            )
            .map_err(|e| format!("failed to fetch netcoredbg release: {}", e))?;

            // Windows uses .zip, Unix platforms use .tar.gz
            let (current_platform, _) = zed::current_platform();
            let extension = match current_platform {
                zed::Os::Windows => "zip",
                _ => "tar.gz",
            };

            // Build the asset name we're looking for
            let asset_name = format!("netcoredbg-{}.{}", platform, extension);

            // Find the matching asset
            let asset = release
                .assets
                .iter()
                .find(|asset| asset.name == asset_name)
                .ok_or_else(|| {
                    format!(
                        "no compatible netcoredbg asset found for platform '{}'. available: [{}]",
                        platform,
                        release
                            .assets
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;

            Ok(asset.download_url.clone())
        })
        .get_binary_path(|version_dir: &str| {
            use zed_extension_api as zed;
            let (platform, _) = zed::current_platform();
            let binary_name = match platform {
                zed::Os::Windows => "netcoredbg.exe",
                _ => "netcoredbg",
            };
            format!("{}/{}", version_dir, binary_name)
        })
        .binary_name_for_logging("netcoredbg")
        .build()
}

/// Create a configuration for VS Code C# extension
pub fn vscode_csharp_config() -> VersionDirConfig {
    VersionConfigBuilder::new("vscode-csharp", "dotnet/vscode-csharp")
        .get_platform_string(|| {
            use zed_extension_api as zed;
            let (platform, arch) = zed::current_platform();
            let platform_str = match (platform, arch) {
                (zed::Os::Mac, zed::Architecture::Aarch64) => "darwin-arm64",
                (zed::Os::Mac, zed::Architecture::X8664) => "darwin-x64",
                (zed::Os::Linux, zed::Architecture::Aarch64) => "linux-arm64",
                (zed::Os::Linux, zed::Architecture::X8664) => "linux-x64",
                (zed::Os::Windows, zed::Architecture::Aarch64) => "win32-arm64",
                (zed::Os::Windows, zed::Architecture::X8664) => "win32-x64",
                _ => return Err("unsupported platform/architecture combination".to_string()),
            };
            Ok(platform_str.to_string())
        })
        .get_download_url(|version: &str, platform: &str| {
            // For VSCode C# extension, we only use the version from GitHub releases
            // The actual download happens through the VSCode gallery API
            Ok(format!(
                "https://ms-dotnettools.gallery.vsassets.io/_apis/public/gallery/publisher/ms-dotnettools/extension/csharp/{}/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage?redirect=true&targetPlatform={}",
                version, platform
            ))
        })
        .get_binary_path(|version_dir: &str| {
            use zed_extension_api as zed;
            let (platform, _) = zed::current_platform();
            let binary_name = match platform {
                zed::Os::Windows => "Microsoft.CodeAnalysis.LanguageServer.exe",
                _ => "Microsoft.CodeAnalysis.LanguageServer",
            };
            format!("{}/extension/.roslyn/{}", version_dir, binary_name)
        })
        .binary_name_for_logging("Roslyn")
        .build()
}
