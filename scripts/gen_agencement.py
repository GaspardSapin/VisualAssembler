#!/usr/bin/env python3
"""Generate (or check) AGENCEMENT.md: the annotated file tree of the project.

Hand-written annotations are preserved across regenerations; only new entries appear
marked with "<!-- annotate -->".

Usage:
    python gen_agencement.py <root> [--check] [--max-depth N] [--output FILE]

--check writes nothing and exits with code 1 if the file is out of sync.

Paths ignored by git (asked once with `git ls-files --others --ignored
--exclude-per-directory=.gitignore --directory -z`) are left out, so local outputs such as
bench/results/ never make the tree machine-dependent. Only the repository's own .gitignore files
count: core.excludesFile and .git/info/exclude are deliberately not read, since they differ from
one machine to the next. When git is missing or the root is not in a repository, a note goes to
stderr and only the built-in exclusions below apply.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

IGNORED_DIRS = {
    ".git", ".hg", ".svn", "node_modules", "venv", ".venv", "env",
    "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache", ".cache",
    "dist", "build", "target", ".next", ".nuxt", "out", "coverage", "htmlcov",
    ".idea", ".vscode", ".gradle", ".tox", "site-packages", ".terraform",
}
IGNORED_FILE_SUFFIXES = (".pyc", ".pyo", ".so", ".o", ".class", ".log", ".lock")
IGNORED_FILES = {".DS_Store", "Thumbs.db"}
TODO = "<!-- annotate -->"

BRANCH, LAST, PIPE, BLANK = "├── ", "└── ", "│   ", "    "
LINE_RE = re.compile(r"^(?P<indent>(?:│   |    )*)(?P<conn>├── |└── )(?P<rest>.*)$")
# Shared with check_project_docs.py (through load_git_ignored). `--exclude-per-directory` reads
# only the committed .gitignore files, unlike `--exclude-standard` (spec section 17.11, D12).
GIT_IGNORED_COMMAND = [
    "git", "ls-files", "--others", "--ignored", "--exclude-per-directory=.gitignore",
    "--directory", "-z",
]
GIT_TIMEOUT_SECS = 30


@dataclass(frozen=True)
class IgnoredPaths:
    """Paths that git ignores below a root: POSIX, relative to the root, no trailing slash.

    An empty set skips nothing, which is the behaviour when git cannot be asked.
    """

    entries: frozenset[str] = frozenset()

    def covers(self, rel: str) -> bool:
        """Return True when `rel` (POSIX, relative) or one of its parent folders is ignored."""
        if not self.entries:
            return False
        parts = rel.split("/")
        return any("/".join(parts[:i]) in self.entries for i in range(1, len(parts) + 1))


def load_git_ignored(root: Path) -> IgnoredPaths:
    """Ask git once which untracked paths below `root` the repository's .gitignore files exclude.

    A fully ignored folder is reported once (git's `--directory`), not file by file.
    Per-user ignore sources (core.excludesFile, .git/info/exclude) are not consulted.
    When git is missing, the root is not inside a work tree, or git fails, a note is
    printed on stderr and an empty set is returned, so callers keep the built-in
    exclusions only.
    """
    try:
        result = subprocess.run(
            GIT_IGNORED_COMMAND, cwd=root, capture_output=True,
            timeout=GIT_TIMEOUT_SECS, check=False,
        )
    except FileNotFoundError:
        reason = "git is not installed or not on PATH"
    except (OSError, subprocess.SubprocessError) as exc:
        reason = f"git could not run: {exc}"
    else:
        if result.returncode == 0:
            names = result.stdout.decode("utf-8", "surrogateescape").split("\0")
            return IgnoredPaths(frozenset(n.rstrip("/") for n in names if n.rstrip("/")))
        lines = result.stderr.decode("utf-8", "replace").strip().splitlines()
        reason = lines[0] if lines else f"git exited with code {result.returncode}"
    print(
        f"note: gitignored paths are not skipped under {root} ({reason})",
        file=sys.stderr,
    )
    return IgnoredPaths()


def keep(entry: Path) -> bool:
    """Return True when the entry belongs in the tree."""
    if entry.name in IGNORED_FILES:
        return False
    if entry.is_dir():
        return entry.name not in IGNORED_DIRS and not entry.name.endswith(".egg-info")
    return not entry.name.endswith(IGNORED_FILE_SUFFIXES)


def walk(
    root: Path, max_depth: int, ignored: IgnoredPaths | None = None,
) -> list[tuple[str, str, bool]]:
    """Return [(display prefix, POSIX relative path, is_dir)] for the whole tree.

    Entries ignored by git are left out. `ignored` defaults to asking git once
    (`load_git_ignored`); callers that already asked pass their result.
    """
    skip = load_git_ignored(root) if ignored is None else ignored
    rows: list[tuple[str, str, bool]] = []

    def rec(directory: Path, prefix: str, depth: int) -> None:
        if depth > max_depth:
            return
        try:
            entries = sorted(
                (
                    e for e in directory.iterdir()
                    if keep(e) and not skip.covers(e.relative_to(root).as_posix())
                ),
                key=lambda e: (not e.is_dir(), e.name.lower()),
            )
        except PermissionError:
            return
        for i, entry in enumerate(entries):
            last = i == len(entries) - 1
            rel = entry.relative_to(root).as_posix()
            rows.append((prefix + (LAST if last else BRANCH), rel, entry.is_dir()))
            if entry.is_dir():
                rec(entry, prefix + (BLANK if last else PIPE), depth + 1)

    rec(root, "", 1)
    return rows


def read_annotations(path: Path) -> dict[str, str]:
    """Read existing annotations, indexed by relative path."""
    if not path.exists():
        return {}
    annotations: dict[str, str] = {}
    stack: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = LINE_RE.match(line)
        if not m:
            continue
        depth = len(m.group("indent")) // 4
        rest = m.group("rest")
        name, _, comment = rest.partition("#")
        name = name.strip()
        comment = comment.strip()
        is_dir = name.endswith("/")
        name = name.rstrip("/")
        del stack[depth:]
        stack.append(name)
        if comment and comment != TODO:
            annotations["/".join(stack)] = comment
        if not is_dir:
            stack.pop()
    return annotations


def render(root: Path, rows, annotations: dict[str, str]) -> str:
    """Render the Markdown document for the given tree."""
    body = [f"{root.name}/"]
    labels = [(pre + Path(rel).name + ("/" if d else ""), rel) for pre, rel, d in rows]
    width = max((len(lbl) for lbl, _ in labels), default=0)
    missing = 0
    for (label, rel) in labels:
        note = annotations.get(rel)
        if note is None:
            note = TODO
            missing += 1
        body.append(f"{label.ljust(width)}  # {note}")
    tree = "\n".join(body)
    header = (
        "# Project layout\n\n"
        "Generated by `python scripts/gen_agencement.py .`.\n"
        "Hand-written annotations are preserved; fill in the entries marked "
        f"`{TODO}` before closing the task.\n\n"
    )
    footer = f"\n\n_{len(rows)} entries · {missing} to annotate._\n"
    return header + "```\n" + tree + "\n```" + footer


def structure_of(text: str) -> list[str]:
    """Extract the structural part of a tree, ignoring annotations."""
    out = []
    for line in text.splitlines():
        m = LINE_RE.match(line)
        if m:
            out.append(m.group("indent") + m.group("conn") + m.group("rest").split("#")[0].strip())
    return out


def main() -> int:
    """Generate or check AGENCEMENT.md for the root given on the command line."""
    ap = argparse.ArgumentParser(description="Generate AGENCEMENT.md")
    ap.add_argument("root", nargs="?", default=".")
    ap.add_argument("--check", action="store_true", help="verify without writing")
    ap.add_argument("--max-depth", type=int, default=6)
    ap.add_argument("--output", default=None)
    args = ap.parse_args()

    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"Root not found: {root}", file=sys.stderr)
        return 2
    target = Path(args.output) if args.output else root / "AGENCEMENT.md"

    # The output file must appear in its own tree: creating it before the walk
    # avoids a spurious out-of-sync report on the very first generation.
    if not args.check and not target.exists():
        target.touch()

    rows = walk(root, args.max_depth)
    content = render(root, rows, read_annotations(target))

    if args.check:
        if not target.exists():
            print("AGENCEMENT.md is missing.")
            return 1
        if structure_of(target.read_text(encoding="utf-8")) != structure_of(content):
            print("AGENCEMENT.md is out of sync with the real file tree.")
            return 1
        print("AGENCEMENT.md is up to date.")
        return 0

    # newline="\n" keeps LF on every platform (spec section 3.4): Windows text mode would
    # otherwise turn each "\n" into "\r\n" and leave a freshly checked-out tree dirty.
    # open() rather than Path.write_text, whose newline argument needs Python >= 3.10.
    with target.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(content)
    print(f"{target} written ({len(rows)} entries).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
