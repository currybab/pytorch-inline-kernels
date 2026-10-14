//! Workspace configuration for the basedpyright wrapper.
//!
//! Mirrors what Zed's built-in basedpyright adapter sends for the "Python"
//! language. Zed answers `workspace/configuration` by looking each requested
//! section up as a literal key, so dotted sections such as
//! `basedpyright.analysis` are top-level keys here, exactly as Zed does it.

use zed_extension_api::serde_json::{Map, Value};

/// Virtual environment directories probed at the worktree root, in order.
pub const VENV_DIR_NAMES: [&str; 2] = [".venv", "venv"];

/// Keys that mean the user already chose an interpreter.
const INTERPRETER_KEYS: [&str; 5] = [
    "venvPath",
    "venv",
    "python",
    "pythonPath",
    "defaultInterpreterPath",
];

const ANALYSIS_SECTION: &str = "basedpyright.analysis";
const DISABLE_ORGANIZE_IMPORTS: &str = "basedpyright.disableOrganizeImports";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualEnv {
    /// Directory name relative to the worktree root, e.g. `.venv`.
    pub dir_name: String,
    /// Absolute path of the interpreter inside the environment.
    pub interpreter: String,
}

impl VirtualEnv {
    pub fn at_root(worktree_root: &str, dir_name: &str, windows: bool) -> Self {
        let relative = if windows {
            "Scripts\\python.exe"
        } else {
            "bin/python"
        };
        let separator = if windows { "\\" } else { "/" };
        let root = worktree_root.trim_end_matches(['/', '\\']);
        Self {
            dir_name: dir_name.to_owned(),
            interpreter: format!("{root}{separator}{dir_name}{separator}{relative}"),
        }
    }
}

/// Builds the workspace configuration from the user's settings, adding the
/// detected virtual environment and Zed's defaults wherever the user has not
/// set a value. Never modifies `user_settings`.
pub fn basedpyright_workspace_config(
    user_settings: Option<&Value>,
    venv: Option<&VirtualEnv>,
) -> Value {
    let user = match user_settings {
        Some(Value::Object(object)) => object.clone(),
        _ => Map::new(),
    };
    let with_venv = match venv {
        Some(venv) if !has_interpreter_setting(&user) => insert_venv(user, venv),
        _ => user,
    };
    Value::Object(insert_defaults(with_venv))
}

fn has_interpreter_setting(settings: &Map<String, Value>) -> bool {
    INTERPRETER_KEYS
        .iter()
        .any(|key| settings.contains_key(*key))
}

fn insert_venv(settings: Map<String, Value>, venv: &VirtualEnv) -> Map<String, Value> {
    let interpreter = Value::String(venv.interpreter.clone());
    let python = Map::from_iter([
        ("pythonPath".to_owned(), interpreter.clone()),
        ("defaultInterpreterPath".to_owned(), interpreter),
    ]);
    let mut result = settings;
    result.insert("venvPath".to_owned(), Value::String(".".to_owned()));
    result.insert("venv".to_owned(), Value::String(venv.dir_name.clone()));
    result.insert("python".to_owned(), Value::Object(python));
    result
}

fn insert_defaults(settings: Map<String, Value>) -> Map<String, Value> {
    let mut result = settings;
    // basedpyright defaults to strict checking; Zed tones it down to "standard".
    let analysis = match result.get(ANALYSIS_SECTION) {
        Some(Value::Object(existing)) if existing.contains_key("typeCheckingMode") => {
            Value::Object(existing.clone())
        }
        Some(Value::Object(existing)) => {
            let mut analysis = existing.clone();
            analysis.insert("typeCheckingMode".to_owned(), "standard".into());
            Value::Object(analysis)
        }
        Some(other) => other.clone(),
        None => Value::Object(Map::from_iter([(
            "typeCheckingMode".to_owned(),
            "standard".into(),
        )])),
    };
    result.insert(ANALYSIS_SECTION.to_owned(), analysis);
    // Ruff organizes imports, as with Zed's built-in Python setup.
    result
        .entry(DISABLE_ORGANIZE_IMPORTS)
        .or_insert(Value::Bool(true));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use zed_extension_api::serde_json::json;

    fn venv() -> VirtualEnv {
        VirtualEnv::at_root("/work/proj/", ".venv", false)
    }

    #[test]
    fn interpreter_path_is_built_per_platform() {
        assert_eq!(venv().interpreter, "/work/proj/.venv/bin/python");
        let windows = VirtualEnv::at_root("C:\\proj", "venv", true);
        assert_eq!(windows.interpreter, "C:\\proj\\venv\\Scripts\\python.exe");
    }

    #[test]
    fn empty_settings_get_venv_and_defaults() {
        let config = basedpyright_workspace_config(None, Some(&venv()));
        assert_eq!(
            config,
            json!({
                "venvPath": ".",
                "venv": ".venv",
                "python": {
                    "pythonPath": "/work/proj/.venv/bin/python",
                    "defaultInterpreterPath": "/work/proj/.venv/bin/python",
                },
                "basedpyright.analysis": { "typeCheckingMode": "standard" },
                "basedpyright.disableOrganizeImports": true,
            })
        );
    }

    #[test]
    fn user_interpreter_choice_wins_over_detected_venv() {
        let user = json!({ "python": { "pythonPath": "/opt/py/bin/python" } });
        let config = basedpyright_workspace_config(Some(&user), Some(&venv()));
        assert_eq!(config["python"]["pythonPath"], "/opt/py/bin/python");
        assert!(config.get("venv").is_none());
    }

    #[test]
    fn user_analysis_settings_are_kept() {
        let user = json!({
            "basedpyright.analysis": { "typeCheckingMode": "strict", "diagnosticMode": "workspace" },
            "basedpyright.disableOrganizeImports": false,
        });
        let config = basedpyright_workspace_config(Some(&user), None);
        assert_eq!(
            config["basedpyright.analysis"]["typeCheckingMode"],
            "strict"
        );
        assert_eq!(
            config["basedpyright.analysis"]["diagnosticMode"],
            "workspace"
        );
        assert_eq!(config["basedpyright.disableOrganizeImports"], false);
    }

    #[test]
    fn missing_type_checking_mode_is_filled_in() {
        let user = json!({ "basedpyright.analysis": { "diagnosticMode": "workspace" } });
        let config = basedpyright_workspace_config(Some(&user), None);
        assert_eq!(
            config["basedpyright.analysis"]["typeCheckingMode"],
            "standard"
        );
        assert_eq!(
            config["basedpyright.analysis"]["diagnosticMode"],
            "workspace"
        );
    }

    #[test]
    fn input_settings_are_not_modified() {
        let user = json!({ "extra": 1 });
        let before = user.clone();
        let _ = basedpyright_workspace_config(Some(&user), Some(&venv()));
        assert_eq!(user, before);
    }

    #[test]
    fn non_object_settings_are_treated_as_empty() {
        let config = basedpyright_workspace_config(Some(&json!("bogus")), None);
        assert_eq!(config["basedpyright.disableOrganizeImports"], true);
    }
}
