//! Plumbing shared by the basedpyright and Ruff wrappers.

use zed_extension_api::{
    self as zed,
    settings::{CommandSettings, LspSettings},
    EnvVars, Worktree,
};

/// A wrapper's own server id plus the built-in server it stands in for.
/// User settings are read from `lsp.<own>` first, then `lsp.<builtin>`, so a
/// configuration written for Zed's Python setup carries over unchanged.
#[derive(Debug, Clone, Copy)]
pub struct ServerIds {
    pub own: &'static str,
    pub builtin: &'static str,
}

impl ServerIds {
    pub fn lsp_settings(&self, worktree: &Worktree) -> zed::Result<Option<LspSettings>> {
        self.first_matching(worktree, |s| {
            s.settings.is_some() || s.initialization_options.is_some()
        })
    }

    /// `lsp.<id>.binary`, when the user set a `path` for it.
    pub fn binary_override(&self, worktree: &Worktree) -> zed::Result<Option<CommandSettings>> {
        let settings = self.first_matching(worktree, |s| {
            s.binary.as_ref().is_some_and(|b| b.path.is_some())
        })?;
        Ok(settings.and_then(|s| s.binary))
    }

    fn first_matching(
        &self,
        worktree: &Worktree,
        matches: impl Fn(&LspSettings) -> bool,
    ) -> zed::Result<Option<LspSettings>> {
        for server_id in [self.own, self.builtin] {
            let settings = LspSettings::for_worktree(server_id, worktree)?;
            if matches(&settings) {
                return Ok(Some(settings));
            }
        }
        Ok(None)
    }
}

/// Builds the command for a user-provided `binary` setting. The user's
/// arguments replace the defaults; their env is layered over the shell env.
pub fn override_command(
    binary: CommandSettings,
    default_args: &[&str],
    shell_env: EnvVars,
) -> Option<zed::Command> {
    let path = binary.path?;
    let args = binary
        .arguments
        .unwrap_or_else(|| default_args.iter().map(|arg| (*arg).to_owned()).collect());
    let overrides = binary.env.unwrap_or_default();
    let inherited: EnvVars = shell_env
        .into_iter()
        .filter(|(key, _)| !overrides.contains_key(key))
        .collect();
    let env = inherited.into_iter().chain(overrides).collect();
    Some(zed::Command {
        command: path,
        args,
        env,
    })
}

/// Resolves a path inside the extension's work directory, which is the
/// process's current directory for extensions.
pub fn absolute_path(relative: &str) -> zed::Result<String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to resolve extension work directory: {error}"))?;
    Ok(cwd.join(relative).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> EnvVars {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn override_without_path_is_ignored() {
        let binary = CommandSettings {
            path: None,
            arguments: Some(vec!["x".into()]),
            env: None,
        };
        assert!(override_command(binary, &["--stdio"], env(&[])).is_none());
    }

    #[test]
    fn override_uses_default_args_when_none_given() {
        let binary = CommandSettings {
            path: Some("/opt/bin/ruff".into()),
            arguments: None,
            env: None,
        };
        let command = override_command(binary, &["server"], env(&[("PATH", "/usr/bin")])).unwrap();
        assert_eq!(command.command, "/opt/bin/ruff");
        assert_eq!(command.args, vec!["server".to_owned()]);
        assert_eq!(command.env, env(&[("PATH", "/usr/bin")]));
    }

    #[test]
    fn override_args_replace_defaults_and_env_layers_over_shell() {
        let binary = CommandSettings {
            path: Some("/opt/bin/basedpyright-langserver".into()),
            arguments: Some(vec!["--stdio".into(), "--verbose".into()]),
            env: Some(HashMap::from([("PATH".into(), "/opt/bin".into())])),
        };
        let shell = env(&[("PATH", "/usr/bin"), ("HOME", "/home/me")]);
        let command = override_command(binary, &["--stdio"], shell).unwrap();
        assert_eq!(
            command.args,
            vec!["--stdio".to_owned(), "--verbose".to_owned()]
        );
        let mut resolved = command.env;
        resolved.sort();
        assert_eq!(resolved, env(&[("HOME", "/home/me"), ("PATH", "/opt/bin")]));
    }
}
