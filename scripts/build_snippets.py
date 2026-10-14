#!/usr/bin/env python3
"""Generates the snippet files Zed loads from the sources in snippets/src/.

Zed picks snippets by file stem == language name lowercased, so:
  snippets/cuda.json  <- src/cuda.json + src/torch.json  (CUDA strings and .cu files)
  snippets/c++.json   <- src/torch.json                  (C++ strings and every C++ file)

Run after editing snippets/src/*.json; tests/test_snippets.py fails when the
generated files are stale.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE_DIR = ROOT / "snippets" / "src"
OUTPUT_DIR = ROOT / "snippets"
# Output file name -> source files merged into it, in order.
OUTPUTS: dict[str, tuple[str, ...]] = {
    "cuda.json": ("cuda.json", "torch.json"),
    "c++.json": ("torch.json",),
}


def load_source(name: str) -> dict[str, dict]:
    return json.loads((SOURCE_DIR / name).read_text())


def merge(sources: tuple[str, ...]) -> dict[str, dict]:
    merged: dict[str, dict] = {}
    for source in sources:
        snippets = load_source(source)
        clashes = sorted(merged.keys() & snippets.keys())
        if clashes:
            raise ValueError(f"{source}: snippet names already defined: {clashes}")
        merged = {**merged, **snippets}
    return merged


def render() -> dict[str, str]:
    """Returns {output file name: JSON text} without writing anything."""
    return {
        name: json.dumps(merge(sources), indent=2, ensure_ascii=False) + "\n"
        for name, sources in OUTPUTS.items()
    }


def main() -> None:
    for name, text in render().items():
        (OUTPUT_DIR / name).write_text(text)
        print(f"wrote snippets/{name}")


if __name__ == "__main__":
    main()
