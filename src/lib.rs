//! Language servers for the "Python (load_inline)" language.
//!
//! Zed binds its built-in Python servers to the "Python" language only, so a
//! file switched to "Python (load_inline)" would otherwise lose them. This
//! extension starts basedpyright and Ruff for it instead. The highlighting
//! itself lives entirely in `languages/`.

mod basedpyright;
mod ruff;
mod server;
mod settings;

use zed_extension_api::{self as zed, serde_json::Value, LanguageServerId, Worktree};

#[derive(Default)]
struct LoadInlineExtension {
    basedpyright: basedpyright::Basedpyright,
    ruff: ruff::Ruff,
}

fn unknown_server(id: &LanguageServerId) -> String {
    format!("unknown language server: {}", id.as_ref())
}

impl zed::Extension for LoadInlineExtension {
    fn new() -> Self {
        Self::default()
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<zed::Command> {
        match id.as_ref() {
            basedpyright::SERVER_ID => self.basedpyright.command(id, worktree),
            ruff::SERVER_ID => self.ruff.command(id, worktree),
            _ => Err(unknown_server(id)),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<Value>> {
        match id.as_ref() {
            basedpyright::SERVER_ID => basedpyright::initialization_options(worktree),
            ruff::SERVER_ID => ruff::initialization_options(worktree),
            _ => Err(unknown_server(id)),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<Value>> {
        match id.as_ref() {
            basedpyright::SERVER_ID => basedpyright::workspace_configuration(worktree),
            ruff::SERVER_ID => ruff::workspace_configuration(worktree),
            _ => Err(unknown_server(id)),
        }
    }
}

zed::register_extension!(LoadInlineExtension);
