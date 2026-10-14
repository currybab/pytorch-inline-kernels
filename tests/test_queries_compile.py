"""Every query file must compile against the grammar its language uses."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest
from tree_sitter import Language, Query

from conftest import LANGUAGES_DIR

LANGUAGE_DIRS = sorted(path for path in LANGUAGES_DIR.iterdir() if path.is_dir())


def grammar_name(language_dir: Path) -> str:
    return tomllib.loads((language_dir / "config.toml").read_text())["grammar"]


def query_files(language_dir: Path) -> list[Path]:
    return sorted(language_dir.glob("*.scm"))


@pytest.fixture(scope="module")
def grammars(python_language: Language, cuda_language: Language) -> dict[str, Language]:
    return {"python": python_language, "cuda": cuda_language}


def test_expected_languages_exist() -> None:
    assert {path.name for path in LANGUAGE_DIRS} == {"python-load-inline", "cuda"}


@pytest.mark.parametrize(
    "query_path",
    [path for language_dir in LANGUAGE_DIRS for path in query_files(language_dir)],
    ids=lambda path: f"{path.parent.name}/{path.name}",
)
def test_query_compiles(query_path: Path, grammars: dict[str, Language]) -> None:
    language = grammars[grammar_name(query_path.parent)]
    Query(language, query_path.read_text())


@pytest.mark.parametrize("language_dir", LANGUAGE_DIRS, ids=lambda path: path.name)
def test_language_has_core_queries(language_dir: Path) -> None:
    names = {path.name for path in query_files(language_dir)}
    assert {"highlights.scm", "brackets.scm", "indents.scm", "injections.scm"} <= names
