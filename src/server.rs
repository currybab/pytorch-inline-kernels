//! Plumbing shared by the basedpyright and Ruff wrappers.

use zed_extension_api::{
    self as zed,
    serde_json::Value,
    settings::{CommandSettings, LspSettings},
    EnvVars, Worktree,
};

/// A wrapper's own server id plus the built-in server it stands in for.
/// Each setting is read from `lsp.<own>` first, then `lsp.<builtin>`, field by
/// field, so a configuration written for Zed's Python setup carries over and
/// can be partly overridden under the wrapper's id.
#[derive(Debug, Clone, Copy)]
pub struct ServerIds {
    pub own: &'static str,
    pub builtin: &'static str,
}

impl ServerIds {
    /// `lsp.<id>.settings`.
    pub fn settings(&self, worktree: &Worktree) -> zed::Result<Option<Value>> {
        self.first_set(worktree, |s| s.settings)
    }

    /// `lsp.<id>.initialization_options`.
    pub fn initialization_options(&self, worktree: &Worktree) -> zed::Result<Option<Value>> {
        self.first_set(worktree, |s| s.initialization_options)
    }

    /// `lsp.<id>.binary`, when the user set a `path` for it.
    pub fn binary_override(&self, worktree: &Worktree) -> zed::Result<Option<CommandSettings>> {
        self.first_set(worktree, binary_with_path)
    }

    fn first_set<T>(
        &self,
        worktree: &Worktree,
        field: impl Fn(LspSettings) -> Option<T>,
    ) -> zed::Result<Option<T>> {
        let candidates = [self.own, self.builtin]
            .into_iter()
            .map(|server_id| LspSettings::for_worktree(server_id, worktree));
        first_set_field(candidates, field)
    }
}

fn binary_with_path(settings: LspSettings) -> Option<CommandSettings> {
    settings.binary.filter(|binary| binary.path.is_some())
}

/// Returns `field` of the first candidate that sets it. Candidates are looked
/// up lazily, so later ids are not read once a value is found.
fn first_set_field<T>(
    candidates: impl IntoIterator<Item = zed::Result<LspSettings>>,
    field: impl Fn(LspSettings) -> Option<T>,
) -> zed::Result<Option<T>> {
    for candidate in candidates {
        if let Some(value) = field(candidate?) {
            return Ok(Some(value));
        }
    }
    Ok(None)
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

    use zed_extension_api::serde_json::json;

    use super::*;

    fn lsp(
        settings: Option<Value>,
        init: Option<Value>,
        binary: Option<CommandSettings>,
    ) -> LspSettings {
        LspSettings {
            binary,
            initialization_options: init,
            settings,
        }
    }

    fn binary(path: Option<&str>) -> CommandSettings {
        CommandSettings {
            path: path.map(str::to_owned),
            arguments: None,
            env: None,
        }
    }

    #[test]
    fn each_field_comes_from_the_first_id_that_sets_it() {
        let candidates = || {
            [
                Ok(lsp(Some(json!({"own": 1})), None, None)),
                Ok(lsp(
                    Some(json!({"builtin": 1})),
                    Some(json!({"init": 1})),
                    None,
                )),
            ]
        };

        let settings = first_set_field(candidates(), |s| s.settings).unwrap();
        let init = first_set_field(candidates(), |s| s.initialization_options).unwrap();

        assert_eq!(settings, Some(json!({"own": 1})));
        assert_eq!(init, Some(json!({"init": 1})));
    }

    #[test]
    fn binary_without_path_falls_through_to_the_next_id() {
        let own = lsp(None, None, Some(binary(None)));
        let builtin = lsp(None, None, Some(binary(Some("/opt/bin/ruff"))));

        let found = first_set_field([Ok(own), Ok(builtin)], binary_with_path).unwrap();

        assert_eq!(found.and_then(|b| b.path), Some("/opt/bin/ruff".to_owned()));
    }

    #[test]
    fn nothing_set_gives_none() {
        let found = first_set_field([Ok(lsp(None, None, None))], |s| s.settings).unwrap();
        assert_eq!(found, None);
    }

    #[test]
    fn later_ids_are_not_read_once_a_value_is_found() {
        let own = lsp(Some(json!({"own": 1})), None, None);
        let candidates = [Ok(own), Err("lookup failed".to_owned())];
        assert_eq!(
            first_set_field(candidates, |s| s.settings),
            Ok(Some(json!({"own": 1})))
        );
    }

    #[test]
    fn lookup_errors_are_returned() {
        let candidates = [Err("lookup failed".to_owned())];
        assert_eq!(
            first_set_field(candidates, |s| s.settings),
            Err("lookup failed".to_owned())
        );
    }

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
