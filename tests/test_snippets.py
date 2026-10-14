"""Checks the snippet sources, the generated snippet files and the manifest."""

from __future__ import annotations

import importlib.util
import json
import re
import tomllib
from pathlib import Path
from types import ModuleType

import pytest

from conftest import LANGUAGES_DIR, ROOT

SNIPPETS_DIR = ROOT / "snippets"
ALLOWED_FIELDS = frozenset({"prefix", "body", "description"})
# Zed matches snippet prefixes word by word; keep them to word characters.
PREFIX_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
TABSTOP_PATTERN = re.compile(r"\$(\d+)|\$\{(\d+)")


def load_builder() -> ModuleType:
    spec = importlib.util.spec_from_file_location("build_snippets", ROOT / "scripts" / "build_snippets.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_json(path: Path) -> dict:
    return json.loads(path.read_text())


def generated_files() -> dict[str, dict]:
    return {path.name: read_json(path) for path in sorted(SNIPPETS_DIR.glob("*.json"))}


def braces_balanced(body: str) -> bool:
    depth = 0
    index = 0
    while index < len(body):
        if body.startswith("${", index):
            depth += 1
            index += 2
            continue
        if body[index] == "}" and depth > 0:
            depth -= 1
        index += 1
    return depth == 0


def as_list(value: str | list[str]) -> list[str]:
    return [value] if isinstance(value, str) else value


def test_generated_files_are_up_to_date() -> None:
    builder = load_builder()
    expected = {name: json.loads(text) for name, text in builder.render().items()}
    assert generated_files() == expected, "run: python3 scripts/build_snippets.py"


def test_manifest_lists_exactly_the_generated_files() -> None:
    manifest = tomllib.loads((ROOT / "extension.toml").read_text())
    listed = sorted(Path(path).name for path in manifest["snippets"])
    assert listed == sorted(generated_files())


def test_snippet_files_are_keyed_by_language_id() -> None:
    # Zed picks snippets by file stem == language name lowercased.
    cuda_name = tomllib.loads((LANGUAGES_DIR / "cuda" / "config.toml").read_text())["name"]
    assert {Path(name).stem for name in generated_files()} == {cuda_name.lower(), "C++".lower()}


def test_torch_snippets_are_available_in_cuda_too() -> None:
    files = generated_files()
    assert files["c++.json"]
    assert files["c++.json"].items() <= files["cuda.json"].items()


@pytest.mark.parametrize("file_name", ["cuda.json", "c++.json"])
def test_snippets_are_well_formed(file_name: str) -> None:
    snippets = generated_files()[file_name]
    seen_prefixes: set[str] = set()
    for name, snippet in snippets.items():
        assert set(snippet) == ALLOWED_FIELDS, f"{name}: fields {sorted(snippet)}"
        prefixes = as_list(snippet["prefix"])
        assert prefixes and all(PREFIX_PATTERN.match(p) for p in prefixes), f"{name}: {prefixes}"
        assert not seen_prefixes & set(prefixes), f"{name}: duplicate prefix {prefixes}"
        seen_prefixes |= set(prefixes)
        body = "\n".join(as_list(snippet["body"]))
        assert body.strip(), f"{name}: empty body"
        assert braces_balanced(body), f"{name}: unbalanced ${{...}}"
        assert as_list(snippet["description"])[0].strip(), f"{name}: empty description"


def test_expected_core_snippets_exist() -> None:
    files = generated_files()
    cuda_prefixes = {p for s in files["cuda.json"].values() for p in as_list(s["prefix"])}
    torch_prefixes = {p for s in files["c++.json"].values() for p in as_list(s["prefix"])}
    assert {"threadIdx", "blockIdx", "blockDim", "gridDim", "__syncthreads", "atomicAdd"} <= cuda_prefixes
    assert {"TORCH_CHECK", "data_ptr", "AT_DISPATCH_FLOATING_TYPES", "PYBIND11_MODULE"} <= torch_prefixes
