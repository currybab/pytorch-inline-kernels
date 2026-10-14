"""Checks the load_inline injection rules against tests/fixtures/injections.py."""

from __future__ import annotations

import re
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


@dataclass(frozen=True)
class Expectation:
    line: int
    language: str | None
    node: Node


def parse_fixture(language: Language) -> Node:
    return Parser(language).parse(FIXTURE.read_bytes()).root_node


def collect_injections(language: Language, root: Node) -> tuple[Injection, ...]:
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


def iter_strings(node: Node):
    if node.type == "string":
        yield node
        return
    for child in node.children:
        yield from iter_strings(child)


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


@pytest.fixture(scope="module")
def injections(python_language: Language, fixture_tree: Node) -> tuple[Injection, ...]:
    return collect_injections(python_language, fixture_tree)


def test_fixture_has_expectations(fixture_tree: Node) -> None:
    expectations = collect_expectations(fixture_tree)
    assert len(expectations) >= 30
    assert {e.language for e in expectations} == {"cuda", "cpp", None}


def test_every_marked_string_is_injected_as_expected(
    fixture_tree: Node, injections: tuple[Injection, ...]
) -> None:
    failures = []
    for expectation in collect_expectations(fixture_tree):
        found = [i.language for i in injections if i.within(expectation.node)]
        wanted = [] if expectation.language is None else [expectation.language]
        if found != wanted:
            failures.append(f"line {expectation.line}: expected {wanted}, got {found}")
    assert not failures, "\n".join(failures)


def test_fstring_segments_form_a_single_injection(
    fixture_tree: Node, injections: tuple[Injection, ...]
) -> None:
    fstrings = [s for s in iter_strings(fixture_tree) if s.text.startswith(b"f")]
    assert fstrings, "fixture must contain an f-string kernel"
    matching = [i for i in injections if i.within(fstrings[0])]
    assert len(matching) == 1
    assert len(matching[0].ranges) > 1, "interpolations should split the content ranges"


def test_no_injection_is_reported_twice(injections: tuple[Injection, ...]) -> None:
    ranges = [i.ranges for i in injections]
    assert len(ranges) == len(set(ranges))


def test_zed_builtin_python_injections_are_kept() -> None:
    source = INJECTIONS.read_text()
    assert '(#set! injection.language "comment")' in source
    assert '(#set! injection.language "sql")' in source
