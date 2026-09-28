"""Tests for check_file_length.py: every counting rule, each case in its own git repository.

Run from the repository root: python3 -m unittest scripts/check_file_length_test.py

Every case writes its files into a fresh temporary git repository, adds the tracked ones,
and runs the checker's counting function or its main on that repository. Each expected
count is written as a literal, and each fixture is built so that getting the rule it tests
wrong changes the count.
"""
from __future__ import annotations

import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_file_length  # noqa: E402

OVER = "fn over() {}\n" * 600


class Repository:
    """A temporary git repository holding tracked and untracked fixture files."""

    def __init__(self, tracked: dict[str, bytes | str], untracked: dict[str, str] | None = None):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        for files in (tracked, untracked or {}):
            for name, content in files.items():
                path = self.root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                data = content.encode("utf-8") if isinstance(content, str) else content
                path.write_bytes(data)
        if tracked:
            subprocess.run(["git", "add", "--", *tracked], cwd=self.root, check=True)

    def __enter__(self) -> Repository:
        return self

    def __exit__(self, *exc: object) -> None:
        self.directory.cleanup()

    def run(self) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = check_file_length.main([str(self.root)])
        return code, out.getvalue(), err.getvalue()


def count(name: str, text: str) -> int | None:
    with Repository({name: text}) as repository:
        return check_file_length.count_file(repository.root, name)


class CountingRules(unittest.TestCase):
    def test_blank_and_whitespace_only_lines(self) -> None:
        self.assertEqual(count("src/a.rs", "fn a() {}\n\n   \n\t\n  \t \nfn b() {}\n   "), 2)

    def test_line_comments(self) -> None:
        text = "// plain\n/// outer doc\n//! inner doc\n  // indented\nfn a() {}\n"
        self.assertEqual(count("src/a.rs", text), 1)

    def test_code_with_trailing_comment_counts_once(self) -> None:
        self.assertEqual(count("src/a.rs", "fn a() {} // trailing\nfn b() {} /* also */\n"), 2)

    def test_block_comment_over_several_lines(self) -> None:
        text = "/* one\n   two\n   three */\nfn a() {}\n/* x\n */ fn b() {}\n"
        self.assertEqual(count("src/a.rs", text), 2)

    def test_nested_rust_block_comment(self) -> None:
        text = "fn a() {}\n/* outer /* inner */ still comment\nstill comment */\nfn b() {}\n"
        self.assertEqual(count("src/a.rs", text), 2)

    def test_typescript_block_comment_does_not_nest(self) -> None:
        text = "const a = 1;\n/* outer /* inner */\nconst b = 2;\n"
        self.assertEqual(count("src/a.ts", text), 2)

    def test_comment_markers_in_a_rust_string(self) -> None:
        text = (
            'const A: &str = "// not a comment"; /* real comment\n'
            "still comment */\n"
            'const B: &str = "first\n'
            "// not a comment\n"
            '";\n'
            "fn after() {}\n"
        )
        self.assertEqual(count("src/a.rs", text), 5)

    def test_comment_markers_in_a_typescript_string(self) -> None:
        text = (
            'const a = "// not a comment"; /* real comment\n'
            "still comment */\n"
            "const b = '// not a comment'; /* real comment\n"
            "still comment */\n"
            "const c = 3;\n"
        )
        self.assertEqual(count("src/a.ts", text), 3)

    def test_rust_raw_string_holding_quote_hash_and_slashes(self) -> None:
        text = (
            'const R: &str = r##"holds "# and // inside"##; /* comment\n'
            "comment */\n"
            'const Q: &str = r#"\n'
            "// inside raw\n"
            '"#;\n'
            "fn after() {}\n"
        )
        self.assertEqual(count("src/a.rs", text), 5)

    def test_lifetime_beside_a_char_literal(self) -> None:
        text = (
            "fn f(s: &'static str) -> char { let c = '/'; c } /* comment\n"
            "comment */\n"
            "fn g<'b>(_: &'b u8) -> char { '\"' } /* comment\n"
            "comment */\n"
            "fn h() {}\n"
        )
        self.assertEqual(count("src/a.rs", text), 3)

    def test_typescript_template_literal(self) -> None:
        text = (
            "const a = `http://x ${ `nested ${1} // inner` } // outer */`; /* comment\n"
            "comment */\n"
            "const z = 1;\n"
        )
        self.assertEqual(count("src/a.ts", text), 2)
        text = "const b = `\n// inside the template\n`;\n"
        self.assertEqual(count("src/a.ts", text), 3)

    def test_typescript_regex_literal(self) -> None:
        text = (
            "const r = /\\/\\//; /* comment\n"
            "comment */\n"
            "const s = x / 2; const t = y / 3; /* comment\n"
            "comment */\n"
            "const u = 1;\n"
        )
        self.assertEqual(count("src/a.ts", text), 3)

    def test_unclosed_quote_in_jsx_text_ends_at_its_line(self) -> None:
        text = "const a = <p>Don't // stop</p>;\n// a comment\nconst b = 1;\n"
        self.assertEqual(count("src/a.tsx", text), 2)

    def test_cfg_test_mod_excluded_and_code_after_it_counted(self) -> None:
        text = (
            "fn real() {}\n"
            "\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            '    const S: &str = "}";\n'
            '    const T: &str = "{ {";\n'
            "    // }\n"
            "    fn t() -> char { let c = '}'; c }\n"
            "}\n"
            "\n"
            "fn after() {}\n"
        )
        self.assertEqual(count("src/a.rs", text), 2)


class ExcludedFiles(unittest.TestCase):
    def assert_only_lib_measured(self, files: dict[str, bytes | str], untracked=None) -> None:
        files = dict(files, **{"src/lib.rs": "fn lib() {}\n"})
        with Repository(files, untracked) as repository:
            code, out, _ = repository.run()
        self.assertEqual(code, 0)
        self.assertEqual(out, "file-length: 1 files measured, 0 over the limit\n")

    def test_inner_cfg_test_file_excluded(self) -> None:
        self.assert_only_lib_measured({"src/fixture.rs": "//! A fixture.\n#![cfg(test)]\n" + OVER})

    def test_tests_suffix_file_excluded(self) -> None:
        self.assert_only_lib_measured({"src/store_tests.rs": OVER})

    def test_tests_directory_file_excluded(self) -> None:
        self.assert_only_lib_measured({"tests/x.rs": OVER})

    def test_typescript_test_file_excluded(self) -> None:
        self.assert_only_lib_measured({"web/x.test.ts": OVER.replace("fn over() {}", "f();")})

    def test_untracked_file_ignored(self) -> None:
        self.assert_only_lib_measured({}, {"src/untracked.rs": OVER})


class Boundary(unittest.TestCase):
    def test_exactly_500_code_lines_passes(self) -> None:
        with Repository({"src/big.rs": "fn a() {}\n" * 500}) as repository:
            code, out, _ = repository.run()
        self.assertEqual(code, 0)
        self.assertEqual(out, "file-length: 1 files measured, 0 over the limit\n")

    def test_501_code_lines_fails_naming_the_file(self) -> None:
        with Repository({"src/big.rs": "fn a() {}\n" * 501}) as repository:
            code, out, _ = repository.run()
        self.assertEqual(code, 1)
        self.assertEqual(
            out,
            "src/big.rs: 501 lines of code (limit 500)\n"
            "file-length: 1 files measured, 1 over the limit\n",
        )

    def test_700_raw_lines_with_200_comment_lines_passes(self) -> None:
        text = "// a comment\n" * 100 + "fn a() {}\n" * 500 + "/* block\n" + "   still\n" * 98 + "*/\n"
        self.assertEqual(text.count("\n"), 700)
        with Repository({"src/big.rs": text}) as repository:
            self.assertEqual(check_file_length.count_file(repository.root, "src/big.rs"), 500)
            code, out, _ = repository.run()
        self.assertEqual(code, 0)
        self.assertEqual(out, "file-length: 1 files measured, 0 over the limit\n")

    def test_invalid_utf8_exits_2_naming_the_path(self) -> None:
        with Repository({"src/bad.rs": b"fn a() {}\n\xff\xfe\n"}) as repository:
            code, _, err = repository.run()
        self.assertEqual(code, 2)
        self.assertIn("src/bad.rs: cannot be read as UTF-8", err)


if __name__ == "__main__":
    unittest.main()
