//! basedpyright for "Python (load_inline)".
//!
//! Resolution order: `lsp.<id>.binary.path` from settings, then
//! `basedpyright-langserver` on the worktree's PATH (an activated venv,
//! `uv tool install basedpyright`, ...), then the npm package installed into
//! the extension's work directory, like Zed's built-in adapter.

use std::path::Path;

use zed_extension_api::{
    self as zed, serde_json::Value, LanguageServerId, LanguageServerInstallationStatus, Worktree,
};

use crate::server::{absolute_path, override_command, ServerIds};
use crate::settings::{basedpyright_workspace_config, VirtualEnv, VENV_DIR_NAMES};

pub const SERVER_ID: &str = "load-inline-basedpyright";
const IDS: ServerIds = ServerIds {
    own: SERVER_ID,
    builtin: "basedpyright",
};
const BINARY_NAME: &str = "basedpyright-langserver";
const DEFAULT_ARGS: [&str; 1] = ["--stdio"];
const NPM_PACKAGE: &str = "basedpyright";
const NPM_SERVER_PATH: &str = "node_modules/basedpyright/langserver.index.js";

#[derive(Default)]
pub struct Basedpyright {
    npm_server_checked: bool,
}

impl Basedpyright {
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
        if let Some(binary) = worktree.which(BINARY_NAME) {
            return Ok(zed::Command {
                command: binary,
                args: DEFAULT_ARGS.map(str::to_owned).to_vec(),
                env,
            });
        }
        let server_path = self.npm_server(id)?;
        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![server_path, DEFAULT_ARGS[0].to_owned()],
            env,
        })
    }

    /// Installs or updates the npm package once per session. Any failure is
    /// tolerated as long as a previously installed copy is still on disk, so
    /// the server keeps starting offline.
    fn npm_server(&mut self, id: &LanguageServerId) -> zed::Result<String> {
        let installed = || Path::new(NPM_SERVER_PATH).exists();
        if !self.npm_server_checked || !installed() {
            zed::set_language_server_installation_status(
                id,
                &LanguageServerInstallationStatus::CheckingForUpdate,
            );
            let result = install_latest(id);
            zed::set_language_server_installation_status(
                id,
                &LanguageServerInstallationStatus::None,
            );
            match result {
                Ok(()) => {}
                Err(_) if installed() => {}
                Err(error) => return Err(error),
            }
            self.npm_server_checked = true;
        }
        absolute_path(NPM_SERVER_PATH)
    }
}

fn install_latest(id: &LanguageServerId) -> zed::Result<()> {
    let latest = zed::npm_package_latest_version(NPM_PACKAGE)?;
    let current = zed::npm_package_installed_version(NPM_PACKAGE)?;
    if Path::new(NPM_SERVER_PATH).exists() && current.as_deref() == Some(latest.as_str()) {
        return Ok(());
    }
    zed::set_language_server_installation_status(
        id,
        &LanguageServerInstallationStatus::Downloading,
    );
    zed::npm_install_package(NPM_PACKAGE, &latest)
        .map_err(|error| format!("failed to install {NPM_PACKAGE}@{latest}: {error}"))
}

pub fn initialization_options(worktree: &Worktree) -> zed::Result<Option<Value>> {
    let user = IDS.initialization_options(worktree)?;
    Ok(Some(user.unwrap_or_else(|| {
        zed::serde_json::json!({
            "python": {
                "analysis": {
                    "autoSearchPaths": true,
                    "useLibraryCodeForTypes": true,
                    "autoImportCompletions": true,
                }
            }
        })
    })))
}

pub fn workspace_configuration(worktree: &Worktree) -> zed::Result<Option<Value>> {
    let user = IDS.settings(worktree)?;
    let venv = detect_venv(worktree);
    Ok(Some(basedpyright_workspace_config(
        user.as_ref(),
        venv.as_ref(),
    )))
}

fn detect_venv(worktree: &Worktree) -> Option<VirtualEnv> {
    let (os, _) = zed::current_platform();
    let windows = matches!(os, zed::Os::Windows);
    VENV_DIR_NAMES
        .iter()
        .find(|dir| {
            worktree
                .read_text_file(&format!("{dir}/pyvenv.cfg"))
                .is_ok()
        })
        .map(|dir| VirtualEnv::at_root(&worktree.root_path(), dir, windows))
}
