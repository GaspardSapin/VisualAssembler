#!/usr/bin/env python3
"""Verify the closing ritual of a task.

Checks:
  1. pilot files present at the root;
  2. a README.md in every folder, and every child folder mentioned by its parent README;
  3. dependencies declared in manifests but missing from DEPENDANCE.md or IMPORT.md
     (for Cargo, the root [dependencies] and the external crates of [workspace.dependencies]);
  3b. [deps]: every dependency of every Cargo workspace member uses `workspace = true`
     (architecture rule R6; needs tomllib, i.e. Python >= 3.11, otherwise a warning);
  3c. [lints]: every Cargo workspace member has exactly `[lints] workspace = true`, and the root
     keeps `workspace.lints.rust.unsafe_code = "deny"` (lint part of architecture rule R8;
     same tomllib fallback as [deps]);
  3d. [deps]: every Cargo package below the root is listed in [workspace].members, so cargo
     can never add one silently as an implicit member that escapes checks 3b and 3c
     (spec section 18.6; same walk exclusions as checks 2 and 6);
  3e. [deps]: the root Cargo.toml has no [patch] or [replace] table, which would bypass
     cargo-deny's source checks (spec section 18.6);
  4. AGENCEMENT.md in sync with the real tree;
  5. sensitive .gitignore entries missing;
  6. overloaded folders (too many files to stay readable);
  7. commits that credit an AI tool (co-author / assisted-by / generated-by trailer,
     "generated with" notice or robot emoji; kept in sync with scripts/git-hooks/commit-msg).

Paths ignored by git are skipped when folders are listed (checks 2 and 6) and in the expected
AGENCEMENT.md tree (check 4). They are asked once with `git ls-files --others --ignored
--exclude-per-directory=.gitignore --directory -z`, so only the repository's own .gitignore files
count (never core.excludesFile or .git/info/exclude); without git or outside a repository, a note
goes to stderr and only the built-in exclusions apply.

Usage:
    python check_project_docs.py <root> [--strict]

Exits with code 1 as soon as a problem is found (0 when everything is clean).
--strict also treats warnings as problems.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11: the Cargo checks are skipped with a warning.
    tomllib = None

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_agencement import (  # noqa: E402
    IGNORED_DIRS,
    IgnoredPaths,
    load_git_ignored,
    read_annotations,
    render,
    structure_of,
    walk,
)

ROOT_FILES = [
    "README.md", "CLAUDE.md", "AGENCEMENT.md", "DEPENDANCE.md", "IMPORT.md",
    "SECURITY.md", "Makefile", ".gitignore", ".claudeignore",
]
GITIGNORE_MUSTS = [".env", "__pycache__", "node_modules", "venv", "dist", "build"]
FILES_PER_DIR_WARN = 15   # beyond this, the folder gets hard to read
FILES_PER_DIR_ERROR = 25  # beyond this, it must be split
# Keep the names and trailer keys in sync with scripts/git-hooks/commit-msg (spec sections 17.6
# and 18.2; Python matches the robot emoji as a code point, so no locale setting is needed here).
_AI_NAMES = r"(claude|anthropic|copilot|cursor|gpt|openai|codex|gemini|aider|devin|windsurf|codeium)"
COAUTHOR_RE = re.compile(
    rf"(co-authored-by|assisted-by|generated-by):.*{_AI_NAMES}|generated with .*{_AI_NAMES}|\U0001F916",
    re.IGNORECASE,
)
# The only accepted [lints] table of a workspace member (architecture rule R8).
MEMBER_LINTS = {"workspace": True}
# `git log` arguments of check 7: author name, email and full message of the last 50 commits,
# re-coded to UTF-8 whatever encoding a commit declares.
GIT_LOG_ARGS = ["log", "--encoding=UTF-8", "--format=%an%n%ae%n%B", "-n", "50"]
GIT_LOG_TIMEOUT_SECS = 15
# Dependency tables of a Cargo manifest, also looked up under every `[target.<cfg>]`.
# The underscore spellings are Cargo's legacy aliases: checked so they cannot bypass R6.
CARGO_DEPENDENCY_SECTIONS = (
    "dependencies", "dev-dependencies", "build-dependencies",
    "dev_dependencies", "build_dependencies",
)
GLOB_CHARS = ("*", "?", "[")
# Lower-cased file name of a Cargo manifest. Compared case-insensitively on every OS, so a
# `cargo.toml` saved on a case-insensitive file system is found and results never depend on the host.
CARGO_MANIFEST_NAME = "cargo.toml"
# Tables that make a manifest a package; `project` is Cargo's legacy alias of `package`.
CARGO_PACKAGE_TABLES = ("package", "project")
# Root tables that redirect dependencies to other sources (check 3e).
CARGO_PATCH_TABLES = ("patch", "replace")
IMPLICIT_MEMBER_PROBLEM = "package inside the workspace is not listed in [workspace].members"
PATCH_PROBLEM = (
    "[deps] Cargo.toml: [patch]/[replace] tables are not allowed "
    "(they bypass cargo-deny's source checks)"
)


class ManifestError(Exception):
    """A Cargo manifest that cannot be read, parsed or understood."""


def iter_dirs(root: Path, ignored: IgnoredPaths):
    """Yield every project folder that holds at least one file of its own.

    Folders and files ignored by git do not count. Folders whose every descendant is
    excluded (built-in names, *.egg-info, gitignored) are not even walked.
    """
    candidates: list[Path] = []
    for current, dirnames, _ in os.walk(root):
        base = Path(current)
        kept: list[str] = []
        for name in dirnames:
            path = base / name
            rel = path.relative_to(root)
            candidates.append(path)
            if name in IGNORED_DIRS or name.endswith(".egg-info"):
                continue  # everything below is excluded by the parts test
            if ignored.covers(rel.as_posix()):
                continue  # everything below is ignored by git
            kept.append(name)
        dirnames[:] = kept
    for path in sorted(candidates):
        if not path.is_dir():
            continue
        rel = path.relative_to(root)
        parts = set(rel.parts)
        if parts & IGNORED_DIRS or any(p.endswith(".egg-info") for p in parts):
            continue
        if path.name in IGNORED_DIRS or path.name.startswith("."):
            continue
        if ignored.covers(rel.as_posix()):
            continue
        if not any(own_file(child, root, ignored) for child in path.iterdir()):
            continue  # no file of its own: no README required
        yield path


def own_file(path: Path, root: Path, ignored: IgnoredPaths) -> bool:
    """Return True when `path` is a file that git does not ignore."""
    return path.is_file() and not ignored.covers(path.relative_to(root).as_posix())


def parse_requirements(path: Path) -> set[str]:
    """Return the lower-cased package names of a pip requirements file."""
    names = set()
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = line.split("#")[0].strip()
        if not line or line.startswith("-"):
            continue
        names.add(re.split(r"[\[<>=!~;\s]", line)[0].strip().lower())
    return {n for n in names if n}


def parse_toml(path: Path, sections: list[str]) -> set[str]:
    """Return the dependency names of a pyproject.toml / Cargo.toml table (empty on failure)."""
    try:
        import tomllib
    except ModuleNotFoundError:
        return set()
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return set()
    names: set[str] = set()
    project = data.get("project", {})
    for dep in project.get("dependencies", []) or []:
        names.add(re.split(r"[\[<>=!~;\s]", dep)[0].strip().lower())
    for group in (project.get("optional-dependencies", {}) or {}).values():
        for dep in group:
            names.add(re.split(r"[\[<>=!~;\s]", dep)[0].strip().lower())
    node = data
    for section in sections:
        node = node.get(section, {}) if isinstance(node, dict) else {}
    if isinstance(node, dict):
        names |= {k.lower() for k in node if k.lower() != "python"}
    return {n for n in names if n}


def load_cargo_manifest(path: Path) -> dict:
    """Parse a Cargo manifest with tomllib, which the caller has checked is available.

    Raises ManifestError when the file is missing, unreadable or not valid TOML.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        raise ManifestError(f"cannot be read ({exc})") from exc
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise ManifestError(f"is not valid TOML ({exc})") from exc


def load_root_cargo(root: Path, problems: list[str], warnings: list[str]) -> dict | None:
    """Parse the root Cargo.toml once for the Cargo manifest checks ([deps] and [lints]).

    Returns None when there is no root Cargo.toml, when tomllib is unavailable (one warning
    per skipped check) or when it cannot be parsed (a problem).
    """
    path = root / "Cargo.toml"
    if not path.exists():
        return None
    if tomllib is None:
        warnings.append(
            "[deps] tomllib unavailable (Python >= 3.11 required): [workspace.dependencies] "
            "is not collected and the workspace = true, implicit-member and [patch]/[replace] "
            "checks are skipped"
        )
        warnings.append(
            "[lints] tomllib unavailable (Python >= 3.11 required): the member [lints] and "
            "workspace unsafe_code checks are skipped"
        )
        return None
    try:
        return load_cargo_manifest(path)
    except ManifestError as exc:
        problems.append(f"[deps] Cargo.toml: {exc}")
        return None


def cargo_workspace_dependencies(manifest: dict) -> set[str]:
    """Return the lower-cased external crates pinned in the root [workspace.dependencies].

    Tables with a `path` key are workspace crates, not external ones, and are left out.
    Raises ManifestError when [workspace.dependencies] is not a table. A non-table
    [workspace] gives an empty set here: check_member_dependencies reports it.
    """
    workspace = manifest.get("workspace", {})
    if not isinstance(workspace, dict):
        return set()
    table = workspace.get("dependencies", {})
    if not isinstance(table, dict):
        raise ManifestError("[workspace.dependencies] is not a table")
    return {
        name.lower() for name, spec in table.items()
        if not (isinstance(spec, dict) and "path" in spec)
    }


def collect_dependencies(
    root: Path, cargo: dict | None, problems: list[str],
) -> dict[str, set[str]]:
    """Collect declared dependencies from every manifest found at the root.

    `cargo` is the parsed root Cargo.toml (None when absent or unusable). Its external
    [workspace.dependencies] entries join the root [dependencies]; a malformed
    [workspace.dependencies] is appended to `problems`.
    """
    found: dict[str, set[str]] = {}
    for name in ("requirements.txt", "requirements-dev.txt"):
        p = root / name
        if p.exists():
            found[name] = parse_requirements(p)
    p = root / "pyproject.toml"
    if p.exists():
        found["pyproject.toml"] = parse_toml(p, ["tool", "poetry", "dependencies"])
    p = root / "Cargo.toml"
    if p.exists():
        names = parse_toml(p, ["dependencies"])
        if cargo is not None:
            try:
                names |= cargo_workspace_dependencies(cargo)
            except ManifestError as exc:
                problems.append(f"[deps] Cargo.toml: {exc}")
        found["Cargo.toml"] = names
    p = root / "package.json"
    if p.exists():
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            names = set(data.get("dependencies", {})) | set(data.get("devDependencies", {}))
            found["package.json"] = {n.lower() for n in names}
        except Exception:
            pass
    p = root / "go.mod"
    if p.exists():
        names = set()
        for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
            line = line.strip()
            if line.startswith("require ") or (line and line[0].isalpha() and "/" in line):
                token = line.replace("require ", "").split()[0]
                if "/" in token:
                    names.add(token.lower())
        found["go.mod"] = names
    return found


def string_list(workspace: dict, key: str) -> list[str]:
    """Return `[workspace].<key>` as a list of strings (empty when absent).

    Raises ManifestError when the key is not an array of strings.
    """
    value = workspace.get(key, [])
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        raise ManifestError(f"[workspace].{key} is not an array of strings")
    return value


def workspace_members(root: Path, workspace: dict) -> list[str]:
    """Return the POSIX paths, relative to `root`, of the [workspace].members folders.

    Glob patterns are expanded like Cargo does: only folders match, and [workspace].exclude
    removes matches that no explicit member covers. Raises ManifestError on bad keys.
    """
    patterns = [p.replace("\\", "/") for p in string_list(workspace, "members")]
    excluded = [PurePosixPath(p.replace("\\", "/")) for p in string_list(workspace, "exclude")]
    explicit = [PurePosixPath(p) for p in patterns if not any(c in p for c in GLOB_CHARS)]

    def under(path: PurePosixPath, bases: list[PurePosixPath]) -> bool:
        return any(path == base or base in path.parents for base in bases)

    members: list[str] = []
    for pattern in patterns:
        if not any(c in pattern for c in GLOB_CHARS):
            members.append(PurePosixPath(pattern).as_posix())
            continue
        try:
            matches = sorted(root.glob(pattern))
        except (ValueError, NotImplementedError) as exc:
            raise ManifestError(f"member pattern '{pattern}' cannot be expanded ({exc})") from exc
        for match in matches:
            if not match.is_dir():
                continue
            try:
                rel = PurePosixPath(match.relative_to(root).as_posix())
            except ValueError:
                rel = PurePosixPath(match.as_posix())  # outside the root: keep it absolute
            if under(rel, excluded) and not under(rel, explicit):
                continue
            members.append(rel.as_posix())
    return list(dict.fromkeys(members))


def dependency_tables(manifest: dict) -> list[tuple[str, object]]:
    """Return (section label, table) for every dependency table of a Cargo manifest.

    Covers the top-level sections and their `[target.<cfg>]` variants. Raises
    ManifestError when [target] or one of its entries is not a table.
    """
    tables: list[tuple[str, object]] = [
        (section, manifest[section]) for section in CARGO_DEPENDENCY_SECTIONS
        if section in manifest
    ]
    targets = manifest.get("target", {})
    if not isinstance(targets, dict):
        raise ManifestError("[target] is not a table")
    for cfg, node in targets.items():
        if not isinstance(node, dict):
            raise ManifestError(f"[target.{cfg}] is not a table")
        tables += [
            (f"target.{cfg}.{section}", node[section]) for section in CARGO_DEPENDENCY_SECTIONS
            if section in node
        ]
    return tables


def check_member_dependencies(root: Path, cargo: dict | None) -> list[str]:
    """Rule R6: every dependency of every workspace member must say `workspace = true`.

    `cargo` is the parsed root Cargo.toml; nothing is checked when it is None or has no
    [workspace]. Returns one `[deps]` problem per offending entry or unusable manifest.
    """
    if cargo is None or "workspace" not in cargo:
        return []
    workspace = cargo["workspace"]
    if not isinstance(workspace, dict):
        return ["[deps] Cargo.toml: [workspace] is not a table"]
    try:
        members = workspace_members(root, workspace)
    except ManifestError as exc:
        return [f"[deps] Cargo.toml: {exc}"]
    problems: list[str] = []
    for member in members:
        label = f"{member}/Cargo.toml"
        try:
            manifest = load_cargo_manifest(root / member / "Cargo.toml")
            tables = dependency_tables(manifest)
        except ManifestError as exc:
            problems.append(f"[deps] {label}: {exc}")
            continue
        for section, table in tables:
            if not isinstance(table, dict):
                problems.append(f"[deps] {label}: [{section}] is not a table")
                continue
            for name, spec in sorted(table.items()):
                if not (isinstance(spec, dict) and spec.get("workspace") is True):
                    problems.append(f"[deps] {label}: '{name}' must use workspace = true")
    return list(dict.fromkeys(problems))


def lints_are_inherited(lints: object) -> bool:
    """Return True when a member's parsed `lints` value is exactly `{"workspace": True}`.

    Compares the key set and uses `is True`, so `workspace = 1` (1 == True in Python), an extra
    `[lints.rust]` table or a missing `[lints]` table (None) are all rejected.
    """
    return (
        isinstance(lints, dict)
        and lints.keys() == MEMBER_LINTS.keys()
        and lints["workspace"] is True
    )


def check_member_lints(root: Path, cargo: dict | None) -> list[str]:
    """Rule R8, lint part: every member inherits the workspace lints, which keep unsafe denied.

    `cargo` is the parsed root Cargo.toml; nothing is checked when it is None or has no
    [workspace]. Returns one `[lints]` problem for the root when
    `workspace.lints.rust.unsafe_code` is not exactly "deny", and one per member whose
    `lints` table is not exactly `workspace = true`. A non-table [workspace], unusable
    member keys and unreadable member manifests are not repeated here: check_member_dependencies
    reports them as `[deps]` problems, and main() always runs both checks.
    """
    if cargo is None or "workspace" not in cargo:
        return []
    workspace = cargo["workspace"]
    if not isinstance(workspace, dict):
        return []  # reported by check_member_dependencies
    problems: list[str] = []
    lints = workspace.get("lints")
    rust = lints.get("rust") if isinstance(lints, dict) else None
    if not (isinstance(rust, dict) and rust.get("unsafe_code") == "deny"):
        problems.append('[lints] Cargo.toml: workspace.lints.rust.unsafe_code must stay "deny"')
    try:
        members = workspace_members(root, workspace)
    except ManifestError:
        return problems  # reported by check_member_dependencies
    for member in members:
        try:
            manifest = load_cargo_manifest(root / member / "Cargo.toml")
        except ManifestError:
            continue  # reported by check_member_dependencies
        if not lints_are_inherited(manifest.get("lints")):
            problems.append(
                f"[lints] {member}/Cargo.toml: lints must be exactly 'workspace = true' "
                "(no [lints.*] override)"
            )
    return problems


def path_identity(path: Path) -> str:
    """Return a comparison key for a folder: absolute, symlinks resolved, case-folded on Windows.

    Two spellings of one folder (`./crates/x`, `crates/x/`, `Crates/X` on Windows) give the same
    key, so a member listed with a different spelling still counts as listed.
    """
    return os.path.normcase(os.path.realpath(path))


def iter_nested_manifests(root: Path, ignored: IgnoredPaths, problems: list[str]):
    """Yield every Cargo manifest below `root`, the root manifest itself excluded.

    The walk skips what checks 2 and 6 skip: built-in folder names (`target` included),
    *.egg-info folders and paths ignored by git. Symlinked folders are not followed.
    A folder that cannot be listed could hide a package, so it is appended to `problems`
    as a `[deps]` problem instead of being skipped silently.
    """
    def unreadable(exc: OSError) -> None:
        where = Path(exc.filename) if exc.filename else root
        try:
            rel = where.relative_to(root).as_posix() or "."
        except ValueError:
            rel = where.as_posix()
        problems.append(f"[deps] {rel}/: folder cannot be listed ({exc.strerror or exc})")

    for current, dirnames, filenames in os.walk(root, onerror=unreadable):
        base = Path(current)
        dirnames[:] = sorted(
            name for name in dirnames
            if name not in IGNORED_DIRS
            and not name.endswith(".egg-info")
            and not ignored.covers((base / name).relative_to(root).as_posix())
        )
        if base == root:
            continue  # the root manifest is the workspace itself
        for name in sorted(filenames):
            if name.lower() != CARGO_MANIFEST_NAME:
                continue
            path = base / name
            if not ignored.covers(path.relative_to(root).as_posix()):
                yield path


def check_implicit_members(root: Path, cargo: dict | None, ignored: IgnoredPaths) -> list[str]:
    """Report every package below the root that [workspace].members does not list.

    Cargo silently adds a path dependency inside the workspace folder as an implicit member,
    which would escape the [deps] and [lints] member checks (spec section 18.6). `cargo` is
    the parsed root Cargo.toml; nothing is checked when it is None or has no [workspace].
    A manifest found by the walk that cannot be read or parsed is reported too, since it
    cannot be shown not to be a package. A non-table [workspace] and unusable member keys
    are not repeated here: check_member_dependencies reports them.
    """
    if cargo is None or "workspace" not in cargo:
        return []
    workspace = cargo["workspace"]
    if not isinstance(workspace, dict):
        return []  # reported by check_member_dependencies
    try:
        members = workspace_members(root, workspace)
    except ManifestError:
        return []  # reported by check_member_dependencies
    listed = {path_identity(root / member) for member in members}
    problems: list[str] = []
    for path in iter_nested_manifests(root, ignored, problems):
        if path_identity(path.parent) in listed:
            continue  # a listed member: checks 3b and 3c cover it
        label = path.relative_to(root).as_posix()
        try:
            manifest = load_cargo_manifest(path)
        except ManifestError as exc:
            problems.append(f"[deps] {label}: {exc}")
            continue
        if any(table in manifest for table in CARGO_PACKAGE_TABLES):
            problems.append(f"[deps] {label}: {IMPLICIT_MEMBER_PROBLEM}")
    return problems


def check_patch_tables(cargo: dict | None) -> list[str]:
    """Report a root [patch] (any registry) or [replace] table (spec section 18.6).

    Both redirect dependencies to sources that cargo-deny's source checks never see; a
    future need goes through an Architect decision. `cargo` is the parsed root Cargo.toml;
    nothing is checked when it is None.
    """
    if cargo is not None and any(table in cargo for table in CARGO_PATCH_TABLES):
        return [PATCH_PROBLEM]
    return []


def recent_commit_text(root: Path) -> str | None:
    """Return the author names, emails and messages of the last 50 commits of `root`.

    git is asked to re-code every message to UTF-8 (`--encoding=UTF-8`) and the bytes are
    decoded as UTF-8 here, never with the locale code page: on Windows that code page (cp1252)
    would turn the robot emoji into mojibake that COAUTHOR_RE cannot match. Returns None after
    a `note:` on stderr when git cannot run or fails, e.g. before the first commit.
    """
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *GIT_LOG_ARGS],
            capture_output=True, timeout=GIT_LOG_TIMEOUT_SECS, check=False,
        )
    except FileNotFoundError:
        reason = "git is not installed or not on PATH"
    except (OSError, subprocess.SubprocessError) as exc:
        reason = f"git could not run: {exc}"
    else:
        if result.returncode == 0:
            return result.stdout.decode("utf-8", "replace")
        lines = result.stderr.decode("utf-8", "replace").strip().splitlines()
        reason = lines[0] if lines else f"git exited with code {result.returncode}"
    print(f"note: commit messages are not checked under {root} ({reason})", file=sys.stderr)
    return None


def main() -> int:
    """Run every check on the root given on the command line; return the exit code."""
    ap = argparse.ArgumentParser(description="Verify the closing ritual")
    ap.add_argument("root", nargs="?", default=".")
    ap.add_argument("--strict", action="store_true")
    args = ap.parse_args()
    root = Path(args.root).resolve()

    problems: list[str] = []
    warnings: list[str] = []
    ignored = load_git_ignored(root)
    project_dirs = list(iter_dirs(root, ignored))

    # 1. pilot files
    for name in ROOT_FILES:
        if not (root / name).exists():
            problems.append(f"[pilot] {name} missing at the root")

    # 2. one README per folder
    for directory in project_dirs:
        if not (directory / "README.md").exists():
            problems.append(f"[readme] {directory.relative_to(root).as_posix()}/ has no README.md")

    # 2b. cascade: every child folder must be mentioned in its parent README
    for directory in project_dirs + [root]:
        parent_readme = directory / "README.md"
        if not parent_readme.exists():
            continue
        text = parent_readme.read_text(encoding="utf-8", errors="ignore")
        for child in sorted(directory.iterdir()):
            if not child.is_dir() or child.name in IGNORED_DIRS or child.name.startswith("."):
                continue
            if child.name.endswith(".egg-info") or not (child / "README.md").exists():
                continue
            if not re.search(rf"(?<![\w-]){re.escape(child.name)}(?![\w-])", text):
                rel = parent_readme.relative_to(root).as_posix()
                problems.append(f"[cascade] {rel} does not mention its child {child.name}/")

    # 3. dependencies: addition log (DEPENDANCE.md) and usage map (IMPORT.md)
    cargo = load_root_cargo(root, problems, warnings)
    manifests = collect_dependencies(root, cargo, problems)
    for doc_name, tag in (("DEPENDANCE.md", "dependency"), ("IMPORT.md", "import")):
        doc = root / doc_name
        if not doc.exists():
            continue
        declared = doc.read_text(encoding="utf-8").lower()
        for manifest, names in manifests.items():
            for name in sorted(names):
                if not re.search(rf"(?<![\w-]){re.escape(name)}(?![\w-])", declared):
                    problems.append(f"[{tag}] '{name}' ({manifest}) missing from {doc_name}")

    # 3b. rule R6: workspace members inherit every dependency from [workspace.dependencies]
    problems += check_member_dependencies(root, cargo)

    # 3c. rule R8 (lint part): members inherit the workspace lints; unsafe_code stays denied
    problems += check_member_lints(root, cargo)

    # 3d. no implicit member: every package below the root is listed in [workspace].members
    problems += check_implicit_members(root, cargo, ignored)

    # 3e. no [patch]/[replace] table at the root
    problems += check_patch_tables(cargo)

    # 4. AGENCEMENT.md
    layout = root / "AGENCEMENT.md"
    if layout.exists():
        current = layout.read_text(encoding="utf-8")
        expected = render(root, walk(root, 6, ignored), read_annotations(layout))
        if structure_of(current) != structure_of(expected):
            problems.append("[layout] AGENCEMENT.md is out of sync — rerun gen_agencement.py")
        elif "<!-- annotate -->" in current:
            warnings.append("[layout] some entries are still unannotated")

    # 5. .gitignore
    gitignore = root / ".gitignore"
    if gitignore.exists():
        content = gitignore.read_text(encoding="utf-8")
        missing = [m for m in GITIGNORE_MUSTS if m not in content]
        if missing:
            warnings.append(f"[gitignore] recommended entries missing: {', '.join(missing)}")

    # 6. folder size (files ignored by git do not count)
    for directory in project_dirs + [root]:
        count = sum(1 for c in directory.iterdir() if own_file(c, root, ignored))
        rel = directory.relative_to(root).as_posix() or "."
        if count > FILES_PER_DIR_ERROR:
            problems.append(f"[volume] {rel}/ holds {count} files — split it into sub-folders")
        elif count > FILES_PER_DIR_WARN:
            warnings.append(f"[volume] {rel}/ holds {count} files — plan a split")

    # 7. AI co-signed commits
    if (root / ".git").exists():
        log = recent_commit_text(root)
        if log is not None and COAUTHOR_RE.search(log):
            problems.append(
                "[git] one of the last 50 commits credits an AI tool (co-author, assisted-by "
                "or generated-with mention, or robot emoji) — rewrite the message "
                "(git commit --amend) and install the commit-msg hook (`make hooks`)"
            )
        hook = root / ".git" / "hooks" / "commit-msg"
        if not hook.exists():
            warnings.append("[git] commit-msg hook not installed (`make hooks`)")

    for item in problems:
        print(item)
    for item in warnings:
        print(f"(warning) {item}")

    if not problems and not warnings:
        print("Closing ritual: nothing to report.")
    return 1 if problems or (args.strict and warnings) else 0


if __name__ == "__main__":
    raise SystemExit(main())
