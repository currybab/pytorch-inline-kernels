"""Checks the load_inline injection rules against tests/fixtures/injections.py."""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest
from tree_sitter import Language, Node, Parser, Query, QueryCursor

from conftest import LANGUAGES_DIR, ROOT

FIXTURE = ROOT / "tests" / "fixtures" / "injections.py"
INJECTIONS = LANGUAGES_DIR / "python-load-inline" / "injections.scm"
EXPECT_MARKER = re.compile(rb"EXPECT:(\w+)")
# Injections Zed's own Python queries already provide; not under test here.
UNRELATED_LANGUAGES = frozenset({"comment", "sql"})


@dataclass(frozen=True)
class Injection:
    language: str
    ranges: tuple[tuple[int, int], ...]

    def within(self, node: Node) -> bool:
        return all(node.start_byte <= start and end <= node.end_byte for start, end in self.ranges)

    def touches(self, node: Node) -> bool:
        """True when any range lies inside node; concatenated strings share one injection."""
        return any(node.start_byte <= start and end <= node.end_byte for start, end in self.ranges)


@dataclass(frozen=True)
class Expectation:
    line: int
    language: str | None
    node: Node


def parse_fixture(language: Language) -> Node:
    return Parser(language).parse(FIXTURE.read_bytes()).root_node


def collect_injections(language: Language, root: Node) -> tuple[Injection, ...]:
    """Runs the query with py-tree-sitter."""
    query = Query(language, INJECTIONS.read_text())
    injections = []
    for pattern_index, captures in QueryCursor(query).matches(root):
        nodes = captures.get("injection.content", [])
        injection_language = query.pattern_settings(pattern_index).get("injection.language")
        if not nodes or injection_language in UNRELATED_LANGUAGES:
            continue
        ranges = tuple((node.start_byte, node.end_byte) for node in nodes)
        injections.append(Injection(injection_language, ranges))
    return tuple(injections)


def collect_injections_rust(runner: Path) -> tuple[Injection, ...]:
    """Runs the query with the Rust tree-sitter crate Zed uses (tests/query_runner).

    The two engines can disagree on text predicates over quantified captures,
    and Zed's behavior is the one that matters.
    """
    result = subprocess.run(
        [str(runner), str(INJECTIONS), str(FIXTURE)], capture_output=True, text=True, check=True
    )
    injections = []
    for line in result.stdout.splitlines():
        injection_language, ranges = line.split("\t")
        if injection_language in UNRELATED_LANGUAGES:
            continue
        spans = tuple(tuple(int(n) for n in span.split("-")) for span in ranges.split(","))
        injections.append(Injection(injection_language, spans))
    return tuple(injections)


def iter_nodes(node: Node, node_type: str):
    if node.type == node_type:
        yield node
        return
    for child in node.children:
        yield from iter_nodes(child, node_type)


def iter_strings(node: Node):
    yield from iter_nodes(node, "string")


def alternation_blocks(source: str, opener: str) -> list[str]:
    """Returns each `[...]` that follows `opener`, matched by bracket depth."""
    blocks = []
    start = source.find(opener)
    while start != -1:
        index = start + len(opener) - 1
        depth = 0
        for end in range(index, len(source)):
            depth += {"[": 1, "]": -1}.get(source[end], 0)
            if depth == 0:
                break
        blocks.append(source[index : end + 1])
        start = source.find(opener, end)
    return blocks


def collect_expectations(root: Node) -> tuple[Expectation, ...]:
    expectations = []
    for string in iter_strings(root):
        marker = EXPECT_MARKER.search(string.text)
        if marker is None:
            continue
        expected = marker.group(1).decode()
        language = None if expected == "none" else expected
        expectations.append(Expectation(string.start_point.row + 1, language, string))
    return tuple(expectations)


@pytest.fixture(scope="module")
def fixture_tree(python_language: Language) -> Node:
    return parse_fixture(python_language)


@pytest.fixture(scope="module", params=["py-tree-sitter", "zed-rust-tree-sitter"])
def injections(request: pytest.FixtureRequest, python_language: Language, fixture_tree: Node) -> tuple[Injection, ...]:
    if request.param == "py-tree-sitter":
        return collect_injections(python_language, fixture_tree)
    return collect_injections_rust(request.getfixturevalue("query_runner"))


def test_fixture_has_expectations(fixture_tree: Node) -> None:
    expectations = collect_expectations(fixture_tree)
    assert len(expectations) >= 30
    assert {e.language for e in expectations} == {"cuda", "cpp", None}


def test_every_marked_string_is_injected_as_expected(
    fixture_tree: Node, injections: tuple[Injection, ...]
) -> None:
    failures = []
    for expectation in collect_expectations(fixture_tree):
        found = [i.language for i in injections if i.touches(expectation.node)]
        wanted = [] if expectation.language is None else [expectation.language]
        if found != wanted:
            failures.append(f"line {expectation.line}: expected {wanted}, got {found}")
    assert not failures, "\n".join(failures)


def test_fstring_segments_form_a_single_injection(
    fixture_tree: Node, injections: tuple[Injection, ...]
) -> None:
    fstrings = [s for s in iter_strings(fixture_tree) if s.text.startswith(b"f")]
    assert len(fstrings) >= 3, "fixture must contain f-string kernels"
    for fstring in fstrings:
        segments = [c for c in fstring.named_children if c.type == "string_content"]
        matching = [i for i in injections if i.touches(fstring)]
        assert len(matching) == 1, f"line {fstring.start_point.row + 1}"
        assert matching[0].ranges == tuple((s.start_byte, s.end_byte) for s in segments)


def test_concatenated_pieces_form_a_single_injection(
    fixture_tree: Node, injections: tuple[Injection, ...]
) -> None:
    concatenations = list(iter_nodes(fixture_tree, "concatenated_string"))
    assert len(concatenations) >= 2, "fixture must contain concatenated kernels"
    for concatenation in concatenations:
        matching = [i for i in injections if i.touches(concatenation)]
        assert len(matching) == 1
        assert matching[0].within(concatenation)
        assert len(matching[0].ranges) == len(concatenation.named_children)


def test_wrapped_string_forms_are_identical_in_every_rule() -> None:
    # The `right: [...]` alternation is repeated in each assignment rule; a
    # form added to one copy only would silently apply to some rules.
    blocks = [" ".join(block.split()) for block in alternation_blocks(INJECTIONS.read_text(), "right: [")]
    assert len(blocks) == 6
    assert len(set(blocks)) == 1


def test_no_injection_is_reported_twice(injections: tuple[Injection, ...]) -> None:
    ranges = [i.ranges for i in injections]
    assert len(ranges) == len(set(ranges))


def test_zed_builtin_python_injections_are_kept() -> None:
    source = INJECTIONS.read_text()
    assert '(#set! injection.language "comment")' in source
    assert '(#set! injection.language "sql")' in source
