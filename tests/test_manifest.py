"""Checks extension.toml against what the Zed extension registry requires."""

from __future__ import annotations

import json
import re
import tomllib

from conftest import LANGUAGES_DIR, QUERY_RUNNER_DIR, ROOT, checkout_grammar

MANIFEST = tomllib.loads((ROOT / "extension.toml").read_text())
# Versions Zed 1.23.3 links (its Cargo.lock). Zed 1.21+ ignores the extension's
# python grammar in favor of the native one, and older Zed uses the extension's
# copy for built-in Python too, so the copy must match what the queries target.
ZED_PYTHON_GRAMMAR_VERSION = "0.25.0"
ZED_TREE_SITTER_VERSION = "0.27.0"


def language_configs() -> dict[str, dict]:
    return {
        path.parent.name: tomllib.loads(path.read_text())
        for path in sorted(LANGUAGES_DIR.glob("*/config.toml"))
    }


def test_every_language_grammar_is_declared() -> None:
    # The registry requires it, and zed-extension fails with
    # "grammar not found" otherwise.
    declared = set(MANIFEST["grammars"])
    used = {config["grammar"] for config in language_configs().values()}
    assert used <= declared


def test_python_grammar_matches_the_version_zed_links() -> None:
    checkout = checkout_grammar("python")
    version = json.loads((checkout / "package.json").read_text())["version"]
    assert version == ZED_PYTHON_GRAMMAR_VERSION


def test_query_runner_uses_zeds_tree_sitter_versions() -> None:
    dependencies = tomllib.loads((QUERY_RUNNER_DIR / "Cargo.toml").read_text())["dependencies"]
    assert dependencies["tree-sitter-python"] == f"={ZED_PYTHON_GRAMMAR_VERSION}"
    assert dependencies["tree-sitter"] == f"={ZED_TREE_SITTER_VERSION}"


def test_grammars_are_pinned_to_commits() -> None:
    for name, grammar in MANIFEST["grammars"].items():
        assert re.fullmatch(r"https://github\.com/[\w.-]+/[\w.-]+", grammar["repository"]), name
        assert re.fullmatch(r"[0-9a-f]{40}", grammar["rev"]), name


def test_repository_is_a_public_https_url() -> None:
    assert re.fullmatch(r"https://github\.com/[\w.-]+/[\w.-]+", MANIFEST["repository"])


def test_language_servers_send_the_python_language_id() -> None:
    # Without language_ids Zed sends the lowercased language name,
    # "python (load_inline)", which the servers do not know.
    for server_id, server in MANIFEST["language_servers"].items():
        assert server["language_ids"] == {name: "python" for name in server["languages"]}, server_id
