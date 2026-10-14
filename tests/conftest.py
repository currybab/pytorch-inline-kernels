"""Shared grammar loading for the query tests.

The Python grammar comes from the `tree-sitter-python` wheel, pinned to the
same minor version Zed links natively. The CUDA grammar is built from the exact
repository and revision pinned in `extension.toml`, so the tests exercise the
grammar Zed will compile for the extension.
"""

from __future__ import annotations

import ctypes
import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

import pytest
import tree_sitter_python
from tree_sitter import Language

ROOT = Path(__file__).resolve().parent.parent
LANGUAGES_DIR = ROOT / "languages"
CACHE_DIR = ROOT / "tests" / ".cache"
SHARED_LIB_SUFFIX = ".dylib" if sys.platform == "darwin" else ".so"


def read_pinned_grammar(name: str) -> tuple[str, str]:
    manifest = tomllib.loads((ROOT / "extension.toml").read_text())
    grammar = manifest["grammars"][name]
    return grammar["repository"], grammar["rev"]


def run(cmd: list[str], cwd: Path | None = None) -> None:
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"{' '.join(cmd)} failed:\n{result.stderr}")


def checkout_grammar(name: str) -> Path:
    repository, rev = read_pinned_grammar(name)
    checkout = CACHE_DIR / f"tree-sitter-{name}-{rev}"
    if not (checkout / "src" / "parser.c").exists():
        shutil.rmtree(checkout, ignore_errors=True)
        checkout.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "init", "-q", str(checkout)])
        run(["git", "fetch", "-q", "--depth", "1", repository, rev], cwd=checkout)
        run(["git", "checkout", "-q", "FETCH_HEAD"], cwd=checkout)
    return checkout


def build_grammar(name: str) -> Language:
    checkout = checkout_grammar(name)
    library = checkout / f"{name}{SHARED_LIB_SUFFIX}"
    if not library.exists():
        sources = [str(path) for path in sorted((checkout / "src").glob("*.c"))]
        run(["cc", "-shared", "-fPIC", "-O2", "-I", "src", *sources, "-o", str(library)], cwd=checkout)
    entry_point = getattr(ctypes.cdll.LoadLibrary(str(library)), f"tree_sitter_{name}")
    entry_point.restype = ctypes.c_void_p
    return Language(wrap_in_capsule(entry_point()))


def wrap_in_capsule(pointer: int) -> object:
    """py-tree-sitter takes a `TSLanguage *` wrapped in a named PyCapsule."""
    capsule_new = ctypes.pythonapi.PyCapsule_New
    capsule_new.restype = ctypes.py_object
    capsule_new.argtypes = (ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p)
    return capsule_new(pointer, b"tree_sitter.Language", None)


QUERY_RUNNER_DIR = ROOT / "tests" / "query_runner"
RUSTUP_CARGO_BIN = Path.home() / ".cargo" / "bin"


def build_query_runner() -> Path:
    """Builds the Rust query runner (same tree-sitter crate version as Zed)."""
    env = {**os.environ, "PATH": f"{RUSTUP_CARGO_BIN}{os.pathsep}{os.environ.get('PATH', '')}"}
    result = subprocess.run(
        ["cargo", "build", "-q", "--release"],
        cwd=QUERY_RUNNER_DIR,
        env=env,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"building tests/query_runner failed:\n{result.stderr}")
    return QUERY_RUNNER_DIR / "target" / "release" / "query_runner"


@pytest.fixture(scope="session")
def query_runner() -> Path:
    return build_query_runner()


@pytest.fixture(scope="session")
def python_language() -> Language:
    return Language(tree_sitter_python.language())


@pytest.fixture(scope="session")
def cuda_language() -> Language:
    return build_grammar("cuda")
