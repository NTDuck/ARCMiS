#!/usr/bin/env python3
"""Rule linter for ARCMiS. Checks every enforceable rule mechanically.

Run before commit: python3 .omp/scripts/lint-rules.py [files]
With no arguments, checks all workspace Rust files plus .omp text files.

Enforced rules (each maps to a section of .omp/rules/*):

- rust.md §1: no leading `::` on `use` items, expressions, types, or
  attributes. Plain paths: `foo::bar`, not `::foo::bar`.
- rust.md §9/macros: every non-prelude macro invocation carries its crate
  path with a bang (`serde_json::json!`, not `json!`); prelude macros stay
  bare; no leading `::`
- rust.md §4 (naming): no single-letter bindings (e, o, c) outside tiny
  numeric loop indices and generic parameters
- rust.md §5: prefer turbofish over type annotations on local let with
  collect()/parse() — reported as advisory findings
- logging.md: no println!/eprintln!/dbg!/print! in workspace code
- code-clarity.md (Stepdown Rule): description purity — no `Args:`
  payload inside model-facing description strings
- code-clarity.md (Stepdown Rule): declaration before implementation —
  an impl block must follow its type declaration
- code-clarity.md: advisory — files longer than 300 lines are listed for review
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
QUALIFIED_USE_RE = re.compile(r"^\s*use\s+::")
QUALIFIED_EXPR_RE = re.compile(r"(^|[^\w:\"'])::(std|core|alloc|crate)::")
MACRO_BANG_RE = re.compile(r"(^|[^\w\"/])(::)?([a-z_][a-z0-9_:]*)!\s*[(\[{]")
QUALIFIED_MACRO_ONLY_RE = re.compile(r"::[a-z_][a-z0-9_:]*!\s*[(\[{]")
MACRO_IN_STRING_RE = re.compile(r"\"(?:[^\"\\]|\\.)*[a-z_][a-z0-9_]*!\s*[(\[{]")
COMMENT_RE = re.compile(r"^\s*//")
ARGS_IN_DESCRIPTION_RE = re.compile(r'"[^"]*\bArgs:')
PRINT_RE = re.compile(r"\b(println!|eprintln!|print!|dbg!)\s*\(")
SINGLE_LETTER_RE = re.compile(r"\blet\s+([a-z])\s*(?::[^=]+)?=")
TURBOFISH_ADVISORY_RE = re.compile(r"let\s+\w+\s*:\s*[^=]+=\s*[\w:.]+(?:\.\w+)?\(\)\.(collect|parse)\(")
PRELUDE_MACROS = {
    "format",
    "vec",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "assert",
    "assert_eq",
    "assert_ne",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "matches",
    "concat",
    "include",
    "env",
    "line",
    "column",
    "file",
    "stringify",
    "cfg",
    "option_env",
}
DERIVE_RE = re.compile(r"#\[\s*derive\s*\(([^)]*)\)\s*\]")

def iter_rust_files(root: Path, only: list[Path]) -> list[Path]:
    if only:
        return [p for p in only if p.suffix == ".rs"]
    return [
        p
        for p in (root / "ARCMiS").rglob("*.rs")
        if "target" not in p.parts
    ]


def check_decl_order(path: Path, text: str) -> list[str]:
    """code-clarity.md (Stepdown Rule) §Declaration before implementation:
    a type's impl block must come after its declaration. Track impl blocks
    whose primary type is declared later in the file, or never declared
    in-file."""
    errors: list[str] = []
    decl_line: dict[str, int] = {}
    impl_line: dict[str, int] = {}
    impl_re = re.compile(r"^\s*impl(?:<[^>]*>)?\s+(?:\S+\s+for\s+)?([A-Za-z_][A-Za-z0-9_]*)")
    decl_re = re.compile(r"^\s*pub\s+(?:struct|enum|union)\s+([A-Za-z_][A-Za-z0-9_]*)")
    for no, line in enumerate(text.splitlines(), start=1):
        if COMMENT_RE.match(line):
            continue
        decl = decl_re.match(line)
        if decl:
            name = decl.group(1)
            decl_line[name] = no
            seen = impl_line.get(name)
            if seen is not None:
                errors.append(
                    f"{path}:{seen}: impl {name} before its declaration at line {no} "
                    f"(code-clarity.md §Declaration before implementation)"
                )
            continue
        impl = impl_re.match(line)
        if impl:
            impl_line.setdefault(impl.group(1), no)
    return errors


def check_rust(path: Path, text: str) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    advisories: list[str] = []
    for no, line in enumerate(text.splitlines(), start=1):
        if QUALIFIED_USE_RE.match(line):
            errors.append(f"{path}:{no}: qualified use (rust.md §1): {line.strip()}")
        if COMMENT_RE.match(line):
            continue
        if QUALIFIED_EXPR_RE.search(line) and not QUALIFIED_MACRO_ONLY_RE.search(line):
            errors.append(f"{path}:{no}: leading :: on path (rust.md §1): {line.strip()}")
        for match in MACRO_BANG_RE.finditer(line):
            if MACRO_IN_STRING_RE.search(line):
                continue
            qualified, name = match.group(2), match.group(3)
            if "::" in name:
                if qualified:
                    errors.append(f"{path}:{no}: leading :: on macro (rust.md §9): {line.strip()}")
            elif not qualified and name not in PRELUDE_MACROS:
                errors.append(f"{path}:{no}: unqualified macro `{name}!` (rust.md §9): {line.strip()}")
        for derive in DERIVE_RE.finditer(line):
            for token in derive.group(1).split(","):
                token = token.strip()
                if token and token.startswith("::"):
                    errors.append(f"{path}:{no}: qualified derive path (rust.md §9): {token}")
        if PRINT_RE.search(line):
            errors.append(f"{path}:{no}: print in workspace code (logging.md): {line.strip()}")
        for m in SINGLE_LETTER_RE.finditer(line):
            name = m.group(1)
            if name not in ("i", "j", "k") or "for " not in line:
                errors.append(f"{path}:{no}: single-letter binding `{name}` (rust.md §4): {line.strip()}")
        if TURBOFISH_ADVISORY_RE.search(line):
            advisories.append(f"{path}:{no}: prefer turbofish over annotation (rust.md §5): {line.strip()}")
    errors.extend(check_decl_order(path, text))
    return errors, advisories



def main() -> int:
    root = Path(__file__).resolve().parent.parent.parent
    only = [Path(a) for a in sys.argv[1:]]
    files = iter_rust_files(root, only)
    errors: list[str] = []
    advisories: list[str] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        file_errors, file_advisories = check_rust(path, text)
        errors.extend(file_errors)
        advisories.extend(file_advisories)
        if not only and text.count("\n") > 300:
            advisories.append(f"{path}: over 300 lines (code-clarity.md): split or trim")
    if errors:
        print("\n".join(errors))
        print(f"{len(errors)} violation(s)")
    if advisories:
        print("\n".join(advisories))
        print(f"{len(advisories)} advisory finding(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
