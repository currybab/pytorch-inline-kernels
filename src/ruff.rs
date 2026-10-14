//! Ruff (`ruff server`) for "Python (load_inline)".
//!
//! Resolution order: `lsp.<id>.binary.path` from settings, then `ruff` on the
//! worktree's PATH, then the latest GitHub release downloaded into the
//! extension's work directory.

use std::fs;

use zed_extension_api::{
    self as zed, serde_json::Value, Architecture, DownloadedFileType, GithubReleaseOptions,
    LanguageServerId, LanguageServerInstallationStatus, Os, Worktree,
};

use crate::server::{absolute_path, override_command, ServerIds};

pub const SERVER_ID: &str = "load-inline-ruff";
const IDS: ServerIds = ServerIds {
    own: SERVER_ID,
    builtin: "ruff",
};
const DEFAULT_ARGS: [&str; 1] = ["server"];
const GITHUB_REPO: &str = "astral-sh/ruff";
const DOWNLOAD_DIR_PREFIX: &str = "ruff-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    /// Path of the binary inside the extracted archive.
    pub binary: String,
    pub file_type: DownloadedFileType,
}

/// Maps a platform to ruff's release asset naming
/// (`ruff-aarch64-apple-darwin.tar.gz`, `ruff-x86_64-pc-windows-msvc.zip`, ...).
pub fn release_asset(os: Os, arch: Architecture) -> ReleaseAsset {
    let arch = match arch {
        Architecture::Aarch64 => "aarch64",
        Architecture::X8664 => "x86_64",
        Architecture::X86 => "i686",
    };
    let target = match os {
        Os::Mac => format!("{arch}-apple-darwin"),
        Os::Linux => format!("{arch}-unknown-linux-gnu"),
        Os::Windows => format!("{arch}-pc-windows-msvc"),
    };
    match os {
        Os::Windows => ReleaseAsset {
            name: format!("ruff-{target}.zip"),
            binary: "ruff.exe".to_owned(),
            file_type: DownloadedFileType::Zip,
        },
        Os::Mac | Os::Linux => ReleaseAsset {
            name: format!("ruff-{target}.tar.gz"),
            binary: format!("ruff-{target}/ruff"),
            file_type: DownloadedFileType::GzipTar,
        },
    }
}

#[derive(Default)]
pub struct Ruff {
    cached_binary: Option<String>,
}

impl Ruff {
    pub fn command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        let env = worktree.shell_env();
        if let Some(binary) = IDS.binary_override(worktree)? {
            if let Some(command) = override_command(binary, &DEFAULT_ARGS, env.clone()) {
                return Ok(command);
            }
        }
        let binary = match worktree.which("ruff") {
            Some(binary) => binary,
            None => self.downloaded_binary(id)?,
        };
        Ok(zed::Command {
            command: binary,
            args: DEFAULT_ARGS.map(str::to_owned).to_vec(),
            env,
        })
    }

    fn downloaded_binary(&mut self, id: &LanguageServerId) -> zed::Result<String> {
        if let Some(binary) = self
            .cached_binary
            .as_ref()
            .filter(|b| fs::metadata(b).is_ok())
        {
            return Ok(binary.clone());
        }
        let asset = {
            let (os, arch) = zed::current_platform();
            release_asset(os, arch)
        };
        zed::set_language_server_installation_status(
            id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let result = download_latest(id, &asset);
        zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
        // Offline or rate limited: an earlier download is better than no server.
        let binary = match result {
            Ok(binary) => binary,
            Err(error) => existing_download(&asset).ok_or(error)?,
        };
        // Re-applied every time so an interrupted earlier attempt cannot leave
        // a binary that exists but is not executable.
        zed::make_file_executable(&binary)?;
        let absolute = absolute_path(&binary)?;
        self.cached_binary = Some(absolute.clone());
        Ok(absolute)
    }
}

/// Downloads the latest release unless it is already present and returns the
/// binary's path relative to the work directory.
fn download_latest(id: &LanguageServerId, asset: &ReleaseAsset) -> zed::Result<String> {
    let release = zed::latest_github_release(
        GITHUB_REPO,
        GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;
    let download = release
        .assets
        .iter()
        .find(|candidate| candidate.name == asset.name)
        .ok_or_else(|| {
            format!(
                "{GITHUB_REPO} {} has no asset {}",
                release.version, asset.name
            )
        })?;

    let version_dir = format!("{DOWNLOAD_DIR_PREFIX}{}", release.version);
    let binary = format!("{version_dir}/{}", asset.binary);
    if fs::metadata(&binary).is_ok() {
        return Ok(binary);
    }
    zed::set_language_server_installation_status(
        id,
        &LanguageServerInstallationStatus::Downloading,
    );
    if let Err(error) = zed::download_file(&download.download_url, &version_dir, asset.file_type) {
        // Do not leave a half-extracted directory behind for the next start.
        let _ = fs::remove_dir_all(&version_dir);
        return Err(format!("failed to download {}: {error}", asset.name));
    }
    remove_downloads_except(&version_dir);
    Ok(binary)
}

fn download_dirs() -> Vec<String> {
    let Ok(entries) = fs::read_dir(".") else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(DOWNLOAD_DIR_PREFIX))
        .collect()
}

fn existing_download(asset: &ReleaseAsset) -> Option<String> {
    download_dirs()
        .into_iter()
        .map(|dir| format!("{dir}/{}", asset.binary))
        .find(|binary| fs::metadata(binary).is_ok())
}

/// Best effort: older versions only waste disk space, so failures are ignored.
fn remove_downloads_except(current: &str) {
    download_dirs()
        .into_iter()
        .filter(|dir| dir != current)
        .for_each(|dir| {
            let _ = fs::remove_dir_all(dir);
        });
}

pub fn initialization_options(worktree: &Worktree) -> zed::Result<Option<Value>> {
    Ok(IDS
        .lsp_settings(worktree)?
        .and_then(|s| s.initialization_options))
}

pub fn workspace_configuration(worktree: &Worktree) -> zed::Result<Option<Value>> {
    Ok(IDS.lsp_settings(worktree)?.and_then(|s| s.settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_and_linux_use_tarballs_with_a_nested_binary() {
        let mac = release_asset(Os::Mac, Architecture::Aarch64);
        assert_eq!(mac.name, "ruff-aarch64-apple-darwin.tar.gz");
        assert_eq!(mac.binary, "ruff-aarch64-apple-darwin/ruff");
        assert!(matches!(mac.file_type, DownloadedFileType::GzipTar));

        let linux = release_asset(Os::Linux, Architecture::X8664);
        assert_eq!(linux.name, "ruff-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(linux.binary, "ruff-x86_64-unknown-linux-gnu/ruff");
    }

    #[test]
    fn windows_uses_a_flat_zip() {
        let windows = release_asset(Os::Windows, Architecture::X8664);
        assert_eq!(windows.name, "ruff-x86_64-pc-windows-msvc.zip");
        assert_eq!(windows.binary, "ruff.exe");
        assert!(matches!(windows.file_type, DownloadedFileType::Zip));
    }
}
