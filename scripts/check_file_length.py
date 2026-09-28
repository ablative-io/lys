#!/usr/bin/env python3
"""The file-length gate leg: no tracked code file holds over 500 lines of code (ADR-111).

Usage: check_file_length.py [REPOSITORY_ROOT]

The root defaults to this script's parent's parent. The tracked file list comes from one
`git ls-files -z` at the root; the disk is never walked, so build output, node_modules and
untracked files are never read. Every tracked file ending .rs, .ts, .tsx, .mts or .mjs is
measured, except:

- files under vendor/;
- Rust test code: a file under any tests/ directory, a file named *_tests.rs, a file whose
  first item is the inner attribute #![cfg(test)], and the body of any item annotated
  #[cfg(test)] (with the attribute and the item's head);
- TypeScript test code: a file named *.test.ts, *.test.tsx, *.spec.ts or *.spec.tsx, or one
  under a __tests__ directory.

No generated-file marker exists in the repository's code files, so none is honoured.

A line of code holds at least one character that is not whitespace and not inside a comment.
Comments are // to the end of the line (/// and //! included) and /* ... */, nested in Rust and
not in TypeScript. A comment marker inside a string, raw string, char literal, template
literal or regular-expression literal is code, not a comment.

Each file is read once, as UTF-8, and scanned in one pass. A file that cannot be read or
decoded is reported with its path and the reason and the checker exits 2; it is never
skipped. Otherwise it prints one line per file over the limit, sorted by path, then a
summary line, and exits 1 if any file is over the limit and 0 if none is.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

LIMIT = 500
RUST_SUFFIXES = (".rs",)
SCRIPT_SUFFIXES = (".ts", ".tsx", ".mts", ".mjs")
SCRIPT_TEST_SUFFIXES = (".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx")

RUST_SPECIAL = re.compile(r"[\n/\"'#{}()\[\];,]")
SCRIPT_SPECIAL = re.compile(r"[\n/\"'`{}]")
RUST_STRING_STOP = re.compile(r'[\\"]')
SCRIPT_TEMPLATE_STOP = re.compile(r"[\\`$]")
RUST_BLOCK_STOP = re.compile(r"/\*|\*/")
CFG_TEST = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
INNER_CFG_TEST = re.compile(r"#\s*!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
WORD_TAIL = re.compile(r"[A-Za-z0-9_$]+$")

# After one of these a / starts a regular expression; after anything else it divides.
REGEX_AFTER_CHARS = frozenset("(,=:[!&|?{};+-*%~^")
REGEX_AFTER_WORDS = frozenset(
    "return typeof instanceof in of new delete void throw case do else yield await".split()
)


class Unreadable(Exception):
    """A tracked file that cannot be read or decoded as UTF-8."""


def _is_ident(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


class _Lines:
    """The set of line numbers holding code, with the #[cfg(test)] item state that routes marks."""

    def __init__(self, text: str) -> None:
        self.text = text
        self.line = 0
        self.counted: set[int] = set()
        self.test_depth = 0
        self.pending = False
        self.pending_lines: set[int] = set()
        self.pending_depth = 0

    def mark(self, line: int) -> None:
        if self.test_depth:
            return
        if self.pending:
            self.pending_lines.add(line)
        else:
            self.counted.add(line)

    def span(self, start: int, end: int) -> None:
        """Mark each line of text[start:end] holding a non-whitespace character; advance the line."""
        segments = self.text[start:end].split("\n")
        for offset, segment in enumerate(segments):
            if segment.strip():
                self.mark(self.line + offset)
        self.line += len(segments) - 1

    def skip(self, start: int, end: int) -> None:
        """Pass over a comment: nothing in it is code."""
        self.line += self.text.count("\n", start, end)

    def commit_pending(self) -> None:
        self.pending = False
        self.counted |= self.pending_lines
        self.pending_lines = set()


def _rust_raw_prefix(text: str, pos: int) -> bool:
    """True when the r (or br, cr) before pos opens a raw string rather than ending a word."""
    if pos == 0 or text[pos - 1] != "r":
        return False
    start = pos - 1
    if start > 0 and text[start - 1] in "bc":
        start -= 1
    return start == 0 or not _is_ident(text[start - 1])


def _rust_string_end(text: str, pos: int) -> int:
    """The index after the closing quote of the escaped string whose opening quote is at pos."""
    at = pos + 1
    while True:
        found = RUST_STRING_STOP.search(text, at)
        if found is None:
            return len(text)
        if found.group() == "\\":
            at = found.start() + 2
        else:
            return found.end()


def _rust_block_end(text: str, pos: int) -> int:
    """The index after the */ closing the nested block comment opened at pos."""
    depth = 1
    at = pos + 2
    while depth:
        found = RUST_BLOCK_STOP.search(text, at)
        if found is None:
            return len(text)
        depth += 1 if found.group() == "/*" else -1
        at = found.end()
    return at


def _rust_char_end(text: str, pos: int) -> int:
    """The index after a char literal opened at pos, or 0 when the quote begins a lifetime."""
    if text[pos + 1 : pos + 2] == "\\":
        close = text.find("'", pos + 3)
        return len(text) if close < 0 else close + 1
    if text[pos + 2 : pos + 3] == "'" and text[pos + 1 : pos + 2] not in ("", "\n"):
        return pos + 3
    return 0


def _rust_structure(lines: _Lines, ch: str) -> None:
    """Route one bracket, brace, semicolon or comma through the #[cfg(test)] state."""
    if lines.test_depth:
        if ch == "{":
            lines.test_depth += 1
        elif ch == "}":
            lines.test_depth -= 1
        return
    if lines.pending:
        if ch in "([":
            lines.pending_depth += 1
        elif ch in ")]":
            lines.pending_depth -= 1
        elif lines.pending_depth <= 0:
            if ch == "{":
                lines.pending = False
                lines.pending_lines = set()
                lines.test_depth = 1
                return
            lines.commit_pending()
    lines.mark(lines.line)


def count_rust(text: str) -> int | None:
    """Lines of code in a Rust source, or None when the whole file is #![cfg(test)] test code."""
    lines = _Lines(text)
    pos = 0
    size = len(text)
    while pos < size:
        found = RUST_SPECIAL.search(text, pos)
        stop = size if found is None else found.start()
        lines.span(pos, stop)
        if found is None:
            break
        ch = text[stop]
        pos = stop + 1
        if ch == "\n":
            lines.line += 1
        elif ch == "/" and text.startswith("//", stop):
            end = text.find("\n", stop)
            pos = size if end < 0 else end
        elif ch == "/" and text.startswith("/*", stop):
            pos = _rust_block_end(text, stop)
            lines.skip(stop, pos)
        elif ch in "#\"" and _rust_raw_prefix(text, stop):
            opener = stop
            while opener < size and text[opener] == "#":
                opener += 1
            hashes = opener - stop
            if text[opener : opener + 1] != '"':
                lines.mark(lines.line)
                continue
            close = text.find('"' + "#" * hashes, opener + 1)
            pos = size if close < 0 else close + 1 + hashes
            lines.span(stop, pos)
        elif ch == '"':
            pos = _rust_string_end(text, stop)
            lines.span(stop, pos)
        elif ch == "'":
            end = _rust_char_end(text, stop)
            if end:
                pos = end
                lines.span(stop, pos)
            else:
                lines.mark(lines.line)
        elif ch == "#":
            if not lines.counted and not lines.test_depth and INNER_CFG_TEST.match(text, stop):
                return None
            attribute = None if lines.test_depth else CFG_TEST.match(text, stop)
            if attribute is None:
                lines.mark(lines.line)
                continue
            if not lines.pending:
                lines.pending = True
                lines.pending_depth = 0
            pos = attribute.end()
            lines.span(stop, pos)
        else:
            _rust_structure(lines, ch)
    if lines.pending:
        lines.commit_pending()
    return len(lines.counted)


def _script_line_end(text: str, pos: int, quote: str) -> int:
    """The index after a '...' or "..." string opened at pos; an unclosed one ends at its line.

    Ending an unclosed string at its line keeps a stray apostrophe in JSX text from
    swallowing the lines after it.
    """
    at = pos + 1
    size = len(text)
    while at < size:
        ch = text[at]
        if ch == "\\":
            at += 2
        elif ch == quote:
            return at + 1
        elif ch == "\n":
            return at
        else:
            at += 1
    return size


def _script_regex_end(text: str, pos: int) -> int:
    """The index after the regular expression opened at pos, flags included; 0 if unclosed."""
    at = pos + 1
    size = len(text)
    in_class = False
    while at < size:
        ch = text[at]
        if ch == "\n":
            return 0
        if ch == "\\":
            at += 2
            continue
        if ch == "[":
            in_class = True
        elif ch == "]":
            in_class = False
        elif ch == "/" and not in_class:
            at += 1
            while at < size and (_is_ident(text[at]) or text[at] == "$"):
                at += 1
            return at
        at += 1
    return 0


def _script_template(lines: _Lines, pos: int) -> tuple[int, bool]:
    """Scan template text from pos; return where it stopped and whether a ${ opened there."""
    text = lines.text
    at = pos
    while True:
        found = SCRIPT_TEMPLATE_STOP.search(text, at)
        if found is None:
            lines.span(pos, len(text))
            return len(text), False
        ch = found.group()
        if ch == "\\":
            at = found.start() + 2
        elif ch == "`":
            lines.span(pos, found.end())
            return found.end(), False
        elif text.startswith("${", found.start()):
            lines.span(pos, found.start() + 2)
            return found.start() + 2, True
        else:
            at = found.end()


def _regex_allowed(prev: str) -> bool:
    if not prev:
        return True
    if _is_ident(prev[-1]) or prev[-1] == "$":
        return prev in REGEX_AFTER_WORDS
    return prev in REGEX_AFTER_CHARS


def count_script(text: str) -> int:
    """Lines of code in a TypeScript or JavaScript module."""
    lines = _Lines(text)
    templates: list[int] = []
    prev = ""
    pos = 0
    size = len(text)
    while pos < size:
        found = SCRIPT_SPECIAL.search(text, pos)
        stop = size if found is None else found.start()
        code = text[pos:stop].rstrip()
        if code.strip():
            lines.mark(lines.line)
            tail = WORD_TAIL.search(code)
            prev = tail.group() if tail else code[-1]
        if found is None:
            break
        ch = text[stop]
        pos = stop + 1
        if ch == "\n":
            lines.line += 1
        elif ch == "/" and text.startswith("//", stop):
            end = text.find("\n", stop)
            pos = size if end < 0 else end
        elif ch == "/" and text.startswith("/*", stop):
            end = text.find("*/", stop + 2)
            pos = size if end < 0 else end + 2
            lines.skip(stop, pos)
        elif ch == "/":
            end = _script_regex_end(text, stop) if _regex_allowed(prev) else 0
            pos = end or stop + 1
            lines.span(stop, pos)
            prev = ")" if end else "/"
        elif ch in "'\"":
            pos = _script_line_end(text, stop, ch)
            lines.span(stop, pos)
            prev = ")"
        elif ch == "`" or (ch == "}" and templates and templates[-1] == 0):
            if ch == "}":
                templates.pop()
            lines.mark(lines.line)
            pos, opened = _script_template(lines, stop + 1)
            if opened:
                templates.append(0)
                prev = "{"
            else:
                prev = ")"
        else:
            if templates:
                templates[-1] += 1 if ch == "{" else -1
            lines.mark(lines.line)
            prev = ch
    return len(lines.counted)


def _parts(path: str) -> list[str]:
    return path.split("/")


def is_measured_path(path: str) -> bool:
    """Whether a tracked path is a code file this leg measures, judged by its path alone."""
    parts = _parts(path)
    name = parts[-1]
    directories = parts[:-1]
    if directories and directories[0] == "vendor":
        return False
    if name.endswith(RUST_SUFFIXES):
        return "tests" not in directories and not name.endswith("_tests.rs")
    if name.endswith(SCRIPT_SUFFIXES):
        return "__tests__" not in directories and not name.endswith(SCRIPT_TEST_SUFFIXES)
    return False


def count_file(root: Path, path: str) -> int | None:
    """Lines of code in one tracked file, or None when the whole file is test code."""
    try:
        text = (root / path).read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise Unreadable(f"{path}: cannot be read as UTF-8: {error}") from error
    if path.endswith(RUST_SUFFIXES):
        return count_rust(text)
    return count_script(text)


def tracked_files(root: Path) -> list[str]:
    """The repository's tracked paths, from one git ls-files -z at the root."""
    listing = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    if listing.returncode != 0:
        reason = listing.stderr.decode("utf-8", "replace").strip()
        raise Unreadable(f"{root}: git ls-files failed: {reason}")
    return [path for path in listing.stdout.decode("utf-8").split("\0") if path]


def main(argv: list[str] | None = None) -> int:
    """Measure every tracked code file under the root; print the report; return the exit code."""
    args = sys.argv[1:] if argv is None else argv
    root = Path(args[0]) if args else Path(__file__).resolve().parent.parent
    try:
        paths = sorted(path for path in tracked_files(root) if is_measured_path(path))
    except (Unreadable, OSError, UnicodeDecodeError) as error:
        print(f"file-length: {error}", file=sys.stderr)
        return 2
    measured = 0
    over: list[str] = []
    unreadable: list[str] = []
    for path in paths:
        try:
            count = count_file(root, path)
        except Unreadable as error:
            unreadable.append(str(error))
            continue
        if count is None:
            continue
        measured += 1
        if count > LIMIT:
            over.append(f"{path}: {count} lines of code (limit {LIMIT})")
    for problem in unreadable:
        print(problem, file=sys.stderr)
    for line in over:
        print(line)
    print(f"file-length: {measured} files measured, {len(over)} over the limit")
    if unreadable:
        return 2
    return 1 if over else 0


if __name__ == "__main__":
    sys.exit(main())
