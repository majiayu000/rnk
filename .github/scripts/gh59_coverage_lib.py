"""Fail-closed parsing and provenance helpers for GH-59 coverage evidence."""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import shutil
import stat
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path, PurePosixPath
from typing import Any
from gh59_coverage_source import SourcePolicyError, analyze_rust_source, detect_coverage_control
CHECKER_VERSION = "4"
COLLECTION_SCHEMA = "gh59-coverage-collection-v1"
EVIDENCE_SCHEMA = "gh59-coverage-v2"
LLVM_TYPE = "llvm.coverage.json.export"
LLVM_VERSION = "3.1.0"
LLVM_COV_VERSION = "cargo-llvm-cov 0.8.7"
COBERTURA_VERSION = "2.0.3"
METRIC_KINDS = ("lines", "functions", "instantiations", "regions", "branches", "mcdc")
MAX_COVERAGE_BYTES = 64 * 1024 * 1024
MAX_DOCUMENT_BYTES = 4 * 1024 * 1024
MAX_LLVM_INTEGER = (1 << 64) - 1
MAX_DETAILED_LINES = MAX_COVERAGE_BYTES // 256
SHA_RE = re.compile(r"[0-9a-f]{40}")
HUNK_RE = re.compile(rb"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@(?: |$)")
CONDITION_RE = re.compile(r"(\d+)% \((\d+)/(\d+)\)")
CRITICAL_MODULES = (
    ("identity", "src/reconciler/identity.rs"), ("plan", "src/reconciler/plan.rs"),
    ("incremental_order", "src/layout/engine/incremental_order.rs"),
    ("incremental_apply", "src/layout/engine/incremental.rs"),
)
EXCLUDED_PARTS = {"test", "tests", "example", "examples", "bench", "benches"}
DECLARATION_ONLY_COVERAGE_EXCEPTIONS = {
    "src/layout/mod.rs": "7a220d67132233240dc670f863866083c0a4347bfc2ffd68cb4ded843c2bba78",
    "src/reconciler/mod.rs": "69936ac2d1ae91f174640f71bece856b222debffd32b09b905f216c05fc1b55f",
    "src/renderer/mod.rs": "4769f3f331d6671a9f7e986d47957575906a5a1ee214ef96703d11a2acbfc39d",
}
DIFF_ARGUMENTS = (
    "diff", "--unified=0", "--inter-hunk-context=0", "--no-color", "--text", "--no-indent-heuristic", "--no-relative",
    "--no-ext-diff", "--no-textconv", "--no-renames", "--diff-algorithm=myers",
    "--src-prefix=a/", "--dst-prefix=b/",
)
class CheckError(Exception):
    """A concise, expected coverage-check failure."""
def fail(message: str) -> None:
    raise CheckError(message)
def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()
def canonical_json(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
def safe_path(raw: str | Path, label: str) -> Path:
    path = Path(raw)
    if not path.is_absolute(): fail(f"{label} must be an absolute path")
    current = Path(path.anchor)
    for part in path.parts[1:]:
        if part in (".", ".."): fail(f"{label} must not contain dot path components")
        current /= part
        if current.is_symlink(): fail(f"{label} must not traverse a symlink")
    return path
def read_regular(path: Path, label: str, limit: int) -> tuple[bytes, tuple[Any, ...]]:
    safe_path(path, label)
    try:
        before = path.lstat()
        if not stat.S_ISREG(before.st_mode): fail(f"{label} must be a regular non-symlink file")
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        with os.fdopen(descriptor, "rb") as stream:
            opened = os.fstat(stream.fileno())
            if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino): fail(f"{label} changed while opening")
            data = stream.read(limit + 1)
        after = path.lstat()
    except OSError as error:
        fail(f"{label} could not be read: {error.__class__.__name__}")
    if len(data) > limit: fail(f"{label} exceeds the size limit")
    identity = (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, sha256(data))
    if identity[:4] != (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns): fail(f"{label} changed while reading")
    return data, identity
def require_distinct(paths: list[tuple[str, Path]], allow_missing: bool = False) -> None:
    lexical: set[str] = set()
    inodes: set[tuple[int, int]] = set()
    for label, path in paths:
        safe_path(path, label)
        normalized = os.path.normcase(os.path.abspath(path))
        if normalized in lexical: fail("artifacts, receipt, and evidence must be distinct files")
        lexical.add(normalized)
        try:
            info = path.lstat()
        except FileNotFoundError:
            if allow_missing: continue
            fail(f"{label} does not exist")
        except OSError as error:
            fail(f"{label} path inspection failed: {error.__class__.__name__}")
        if not stat.S_ISREG(info.st_mode): fail(f"{label} must be a regular non-symlink file")
        inode = (info.st_dev, info.st_ino)
        if inode in inodes: fail("artifacts, receipt, and evidence must not alias")
        inodes.add(inode)
def git_launcher() -> str:
    found = shutil.which("git")
    if not found or not Path(found).is_absolute(): fail("git executable is unavailable")
    return found
def git_command(repo: Path, arguments: list[str]) -> list[str]:
    return [git_launcher(), "--no-replace-objects", "-C", str(repo), "-c", "core.quotePath=true", "-c", "core.attributesFile=/dev/null", "-c", "core.fileMode=true", "-c", "core.symlinks=true", "-c", "core.fsmonitor=false", "-c", "core.ignoreStat=false", "-c", "core.untrackedCache=false", *arguments]
def run_git(repo: Path, arguments: list[str], input_data: bytes | None = None) -> bytes:
    environment = {
        "PATH": os.environ.get("PATH", ""), "LC_ALL": "C", "LANG": "C",
        "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_ATTR_NOSYSTEM": "1", "GIT_OPTIONAL_LOCKS": "0", "GIT_NO_REPLACE_OBJECTS": "1",
    }
    try:
        result = subprocess.run(git_command(repo, arguments), input=input_data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False, env=environment)
    except OSError as error:
        fail(f"git execution failed: {error.__class__.__name__}")
    if result.returncode != 0: fail(f"git command failed: {arguments[0]}")
    return result.stdout
def tracked_tree(repo: Path) -> None:
    for option in ("-v", "-f"):
        if any(item and not item.startswith(b"H ") for item in run_git(repo, ["ls-files", option, "-z"]).split(b"\0")): fail("Git index contains hidden state flags")
    algorithm = run_git(repo, ["rev-parse", "--show-object-format"]).decode().strip()
    if algorithm not in ("sha1", "sha256"): fail("Git object format is unsupported")
    for entry in run_git(repo, ["ls-files", "--stage", "-z"]).split(b"\0"):
        if not entry: continue
        try: metadata, raw_path = entry.split(b"\t", 1); mode, expected, stage = metadata.split()
        except ValueError: fail("Git index entry is malformed")
        if stage != b"0": fail("Git index contains an unmerged entry")
        if mode not in (b"100644", b"100755", b"120000"): fail("Git index contains an unsupported tracked type")
        try: decoded = raw_path.decode("utf-8")
        except UnicodeDecodeError: fail("tracked path is not UTF-8")
        relative = PurePosixPath(decoded)
        if relative.is_absolute() or not relative.parts or any(part in (".", "..") for part in relative.parts): fail("tracked path is invalid")
        path = repo.joinpath(*relative.parts)
        safe_path(path.parent, "tracked worktree parent")
        if mode == b"120000":
            try:
                before = path.lstat()
                if not stat.S_ISLNK(before.st_mode): fail("tracked worktree type does not match the index")
                data = os.readlink(os.fsencode(path))
                after = path.lstat()
            except OSError as error:
                fail(f"tracked worktree symlink could not be read: {error.__class__.__name__}")
            identity = lambda item: (item.st_dev, item.st_ino, item.st_mode, item.st_size, item.st_mtime_ns)
            if identity(before) != identity(after): fail("tracked worktree symlink changed while reading")
            if len(data) > MAX_COVERAGE_BYTES: fail("tracked worktree symlink exceeds the size limit")
        else:
            try:
                before = path.lstat()
            except OSError as error:
                fail(f"tracked worktree file could not be inspected: {error.__class__.__name__}")
            data = read_regular(path, "tracked worktree file", MAX_COVERAGE_BYTES)[0]
            try:
                after = path.lstat()
            except OSError as error:
                fail(f"tracked worktree file could not be inspected: {error.__class__.__name__}")
            if (before.st_dev, before.st_ino, before.st_mode) != (after.st_dev, after.st_ino, after.st_mode): fail("tracked worktree mode changed while reading")
            executable = bool(before.st_mode & stat.S_IXUSR)
            if executable != (mode == b"100755"): fail("tracked worktree mode does not match the index")
        digest = hashlib.new(algorithm, b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest().encode()
        if digest != expected: fail("tracked worktree bytes do not match the index")
def require_sha(value: str, label: str) -> None:
    if SHA_RE.fullmatch(value) is None: fail(f"{label} must be a 40-character lowercase SHA")
def repository_snapshot(
    repo_raw: str, head: str, base: str, merge_base: str, trusted_ref: str,
) -> tuple[Path, dict[str, Any], bytes]:
    require_sha(head, "head")
    require_sha(base, "base")
    require_sha(merge_base, "merge-base")
    repo = safe_path(repo_raw, "repo-root")
    tracked_tree(repo)
    if not trusted_ref or trusted_ref.startswith("-") or any(
        token in trusted_ref for token in ("..", "@{", "~", "^", ":", "?", "*", "[", " ", "\t", "\n")
    ):
        fail("trusted-base-ref is invalid")
    full_ref = trusted_ref if trusted_ref.startswith("refs/remotes/") else f"refs/remotes/{trusted_ref}"
    if not full_ref.startswith("refs/remotes/") or run_git(repo, ["check-ref-format", full_ref]) != b"": fail("trusted-base-ref must name a remote-tracking branch")
    if run_git(repo, ["rev-parse", "--show-toplevel"]).decode().strip() != str(repo): fail("repo-root must be the canonical Git worktree root")
    actual_head = run_git(repo, ["rev-parse", "HEAD"]).decode().strip()
    if actual_head != head: fail("HEAD does not match the supplied head")
    if run_git(repo, ["status", "--porcelain=v1", "--untracked-files=all"]): fail("worktree is not clean")
    common_raw = Path(run_git(repo, ["rev-parse", "--git-common-dir"]).decode().strip())
    common = common_raw if common_raw.is_absolute() else repo / common_raw
    if (common / "info/grafts").exists(): fail("legacy Git grafts are forbidden")
    for label, commit in (("head", head), ("base", base), ("merge-base", merge_base)):
        try:
            run_git(repo, ["cat-file", "-e", f"{commit}^{{commit}}"])
        except CheckError:
            fail(f"{label} commit does not exist")
    try:
        resolved = run_git(
            repo, ["rev-parse", "--verify", "--end-of-options", f"{full_ref}^{{commit}}"],
        ).decode().strip()
    except CheckError:
        fail("trusted-base-ref cannot be resolved")
    if resolved != base: fail("base does not match trusted-base-ref")
    merge_bases = run_git(repo, ["merge-base", "--all", head, base]).decode().splitlines()
    if len(merge_bases) != 1: fail("head and base do not have one unique merge-base")
    if merge_bases[0] != merge_base: fail("supplied merge-base does not match Git")
    arguments = [*DIFF_ARGUMENTS, f"{merge_base}...{head}", "--"]
    names_arguments = [
        "diff", "--name-status", "-z", "--no-renames", "--no-ext-diff", "--no-textconv",
        f"{merge_base}...{head}", "--",
    ]
    diff = run_git(repo, arguments)
    names = run_git(repo, names_arguments)
    snapshot = {
        "head_sha": head,
        "base_sha": base,
        "merge_base_sha": merge_base,
        "trusted_base_ref": trusted_ref,
        "commit_timestamp": run_git(repo, ["show", "-s", "--format=%cI", head]).decode().strip(),
        "diff_command": git_command(repo, arguments),
        "diff_sha256": sha256(diff),
        "changed_paths_command": git_command(repo, names_arguments),
        "changed_paths_sha256": sha256(names),
        "git_executable": {"version": run_git(repo, ["--version"]).decode().strip(), "path": git_launcher(), "sha256": sha256(read_regular(Path(git_launcher()).resolve(strict=True), "git executable", MAX_COVERAGE_BYTES)[0])},
    }
    changed_paths: set[str] = set()
    expected = production_paths(names, repo, changed_paths)
    reject_coverage_affecting_changes(changed_paths)
    parse_changed_lines(diff, repo, expected, changed_paths=changed_paths)
    enforce_tracked_head_coverage_policy(repo, head)
    return repo, snapshot, diff
def normalize_source(raw: Any, repo: Path) -> str:
    if not isinstance(raw, str) or not raw or "\x00" in raw or "\\" in raw: fail("coverage contains an invalid source path")
    candidate = Path(raw)
    if candidate.is_absolute():
        try:
            raw = candidate.resolve(strict=False).relative_to(repo).as_posix()
        except ValueError:
            fail("coverage source path escapes repo-root")
    path = PurePosixPath(raw)
    if path.is_absolute() or any(part == ".." for part in path.parts): fail("coverage source path escapes repo-root")
    parts = tuple(part for part in path.parts if part not in ("", "."))
    if not parts: fail("coverage contains an empty normalized source path")
    normalized = PurePosixPath(*parts).as_posix()
    resolved = (repo / normalized).resolve(strict=False)
    try:
        resolved.relative_to(repo)
    except ValueError:
        fail("coverage source path escapes repo-root")
    return normalized
def is_production_rust(path: str) -> bool:
    source = PurePosixPath(path)
    parts = source.parts
    if not path.endswith(".rs") or source.stem in {"test", "tests"} or any(part in EXCLUDED_PARTS for part in parts):
        return False
    if len(parts) >= 2 and parts[0] == "src":
        return True
    return len(parts) >= 4 and parts[0] == "crates" and parts[2] == "src"
def reviewed_declaration_only_change(path: str, added: set[int], bodies: list[bytes]) -> bool:
    expected = DECLARATION_ONLY_COVERAGE_EXCEPTIONS.get(path)
    return expected is not None and len(bodies) == len(added) and sha256(b"\n".join(bodies)) == expected
def production_paths(data: bytes, repo: Path, changed_paths: set[str] | None = None) -> set[str]:
    fields = data.split(b"\0")
    if not fields or fields[-1] != b"": fail("changed-path output is malformed")
    fields.pop()
    if len(fields) % 2: fail("changed-path output is malformed")
    result: set[str] = set()
    for status_raw, path_raw in zip(fields[::2], fields[1::2]):
        if status_raw not in (b"A", b"M", b"D", b"T"): fail("changed-path status is unsupported")
        try:
            path = normalize_source(path_raw.decode("utf-8"), repo)
        except UnicodeDecodeError:
            fail("changed source path is not UTF-8")
        if changed_paths is not None: changed_paths.add(path)
        if status_raw != b"D" and is_production_rust(path):
            result.add(path)
    return result
def coverage_affecting_path(path: str) -> bool:
    source = PurePosixPath(path); name = source.name
    return name in ("build.rs", "Cargo.toml", "Cargo.lock") or name.startswith("rust-toolchain") or (".cargo" in source.parts and name.startswith("config"))
def reject_coverage_affecting_changes(paths: set[str]) -> None:
    if any(coverage_affecting_path(path) for path in paths): fail("coverage-affecting build/tool configuration changed")
def parse_changed_lines(
    diff: bytes, repo: Path, expected_paths: set[str] | None = None, added_bodies: dict[str, list[bytes]] | None = None,
    changed_paths: set[str] | None = None,
) -> dict[str, set[int]]:
    changed: dict[str, set[int]] = {}
    observed: set[str] = set()
    current: str | None = None
    in_file = False
    old_remaining = new_remaining = old_line = new_line = 0
    for line in diff.split(b"\n"):
        if old_remaining or new_remaining:
            prefix = line[:1]
            if prefix == b" ":
                old_remaining -= 1
                new_remaining -= 1
                old_line += 1
                new_line += 1
            elif prefix == b"-":
                old_remaining -= 1
                old_line += 1
            elif prefix == b"+":
                if current is not None:
                    changed.setdefault(current, set()).add(new_line)
                    if added_bodies is not None: added_bodies.setdefault(current, []).append(line[1:])
                new_remaining -= 1
                new_line += 1
            elif line == rb"\ No newline at end of file":
                continue
            else:
                fail("diff hunk body is malformed")
            if old_remaining < 0 or new_remaining < 0: fail("diff hunk counts are inconsistent")
            continue
        if line.startswith(b"diff --git "):
            in_file = True
            current = None
            continue
        if line.startswith(b"Binary files ") or line == b"GIT binary patch": fail("diff contains a binary production candidate")
        if line.startswith((b"--- ", b"+++ ")) and in_file:
            target = line[4:]
            if target == b"/dev/null":
                if line.startswith(b"+++ "): current = None
                continue
            prefix = b"a/" if line.startswith(b"--- ") else b"b/"
            if not target.startswith(prefix) or b"\t" in target or target.startswith(b'"'):
                fail("diff contains an unsupported source path")
            try:
                normalized = normalize_source(target[2:].decode("utf-8"), repo)
            except UnicodeDecodeError:
                fail("diff source path is not UTF-8")
            if changed_paths is not None: changed_paths.add(normalized)
            if line.startswith(b"--- "): continue
            current = normalized if is_production_rust(normalized) else None
            if current is not None:
                observed.add(current)
                changed.setdefault(current, set())
            continue
        hunk = HUNK_RE.match(line)
        if hunk:
            if not in_file: fail("diff hunk appears outside a file")
            old_line = int(hunk.group(1))
            old_remaining = int(hunk.group(2) or b"1")
            new_line = int(hunk.group(3))
            new_remaining = int(hunk.group(4) or b"1")
    if old_remaining or new_remaining: fail("diff ended before its hunk counts were consumed")
    if expected_paths is not None and not expected_paths.issubset(observed): fail("a changed production Rust path lacks a textual diff")
    if not changed or not any(changed.values()): fail("diff contains no added production Rust lines")
    return changed
def duplicate_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result: fail("JSON contains a duplicate object key")
        result[key] = value
    return result
def strict_json(data: bytes, label: str = "LLVM JSON") -> dict[str, Any]:
    if not data.strip(): fail(f"{label} is empty")
    try:
        value = json.loads(
            data,
            object_pairs_hook=duplicate_object,
            parse_constant=lambda _value: fail(f"{label} contains a non-finite number"),
        )
    except (ValueError, UnicodeDecodeError):
        fail(f"{label} is malformed")
    if not isinstance(value, dict): fail(f"{label} root must be an object")
    return value
def integer(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= MAX_LLVM_INTEGER:
        fail(f"{label} must be a non-negative 64-bit integer")
    return value
def metric_counts(item: Any, kind: str, require_notcovered: bool = False) -> tuple[int, int]:
    if not isinstance(item, dict): fail(f"LLVM lacks {kind} summary")
    count = integer(item.get("count"), f"LLVM {kind} count")
    covered = integer(item.get("covered"), f"LLVM {kind} covered")
    percent, notcovered = item.get("percent"), item.get("notcovered")
    invalid_percent = (
        isinstance(percent, bool) or not isinstance(percent, (int, float))
        or percent < 0 or percent > 100 or not math.isfinite(percent)
    )
    if covered > count or invalid_percent: fail(f"LLVM {kind} summary is inconsistent")
    expected = 0.0 if count == 0 else covered / count * 100.0
    if abs(percent - expected) > 1e-9: fail(f"LLVM {kind} summary is inconsistent")
    if require_notcovered or notcovered is not None:
        if integer(notcovered, f"LLVM {kind} notcovered") != count - covered: fail(f"LLVM {kind} notcovered is inconsistent")
    return count, covered
def summary_counts(record: dict[str, Any], kind: str) -> tuple[int, int]:
    summaries = record.get("summary")
    if not isinstance(summaries, dict): fail(f"LLVM file lacks {kind} summary")
    return metric_counts(summaries.get(kind), kind, kind in ("regions", "branches", "mcdc"))
def llvm_vector(value: Any, length: int, label: str) -> list[int]:
    if (
        not isinstance(value, list) or len(value) != length
        or any(isinstance(item, bool) or not isinstance(item, int) or not 0 <= item <= MAX_LLVM_INTEGER for item in value)
    ):
        fail(f"LLVM JSON {label} shape or values are invalid")
    return value
def function_source(raw: Any, repo: Path) -> str | None:
    if not isinstance(raw, str) or not raw or "\x00" in raw or not Path(raw).is_absolute():
        fail("LLVM function source path is invalid")
    resolved = Path(raw).resolve(strict=False)
    try:
        relative = resolved.relative_to(repo)
    except ValueError:
        return None
    return normalize_source(relative.as_posix(), repo)
def head_blob(repo: Path, head: str, path: str) -> bytes:
    require_sha(head, "head")
    source = run_git(repo, ["cat-file", "blob", f"{head}:{path}"])
    expected = run_git(repo, ["rev-parse", f"{head}:{path}"]).decode().strip()
    algorithm = {40: "sha1", 64: "sha256"}.get(len(expected))
    if algorithm is None or hashlib.new(algorithm, b"blob " + str(len(source)).encode() + b"\0" + source).hexdigest() != expected:
        fail("coverage source blob hash disagrees with HEAD")
    return source
def enforce_tracked_head_coverage_policy(repo: Path, head: str) -> None:
    require_sha(head, "head")
    tree = run_git(repo, ["ls-tree", "-rl", "-z", head])
    if len(tree) > MAX_DOCUMENT_BYTES: fail("tracked HEAD tree exceeds the size limit")
    entries: list[tuple[str, bytes, int]] = []; total = 0
    for raw in tree.split(b"\0"):
        if not raw: continue
        try: metadata, path_raw = raw.split(b"\t", 1); mode, kind, oid, size_raw = metadata.split()
        except ValueError: fail("tracked HEAD tree entry is malformed")
        if not path_raw.endswith(b".rs"): continue
        try: path = normalize_source(path_raw.decode("utf-8"), repo); size = int(size_raw)
        except (UnicodeDecodeError, ValueError): fail("tracked HEAD Rust entry is malformed")
        if mode not in (b"100644", b"100755") or kind != b"blob" or size < 0: fail("tracked HEAD Rust entry is not a regular blob")
        entries.append((path, oid, size)); total += size
        if len(entries) > MAX_DETAILED_LINES or total > MAX_COVERAGE_BYTES: fail("tracked HEAD Rust source exceeds the size limit")
    output = run_git(repo, ["cat-file", "--batch"], b"".join(oid + b"\n" for _path, oid, _size in entries))
    if len(output) > MAX_COVERAGE_BYTES + len(entries) * 128: fail("tracked HEAD Rust source exceeds the size limit")
    cursor = 0
    for _path, expected_oid, expected_size in entries:
        end = output.find(b"\n", cursor)
        if end < 0: fail("tracked HEAD Rust blob batch is malformed")
        header = output[cursor:end].split(); start = end + 1; stop = start + expected_size
        if header != [expected_oid, b"blob", str(expected_size).encode()] or stop >= len(output) or output[stop:stop + 1] != b"\n": fail("tracked HEAD Rust blob batch is malformed")
        source = output[start:stop]; algorithm = {40: "sha1", 64: "sha256"}.get(len(expected_oid))
        if algorithm is None or hashlib.new(algorithm, b"blob " + str(len(source)).encode() + b"\0" + source).hexdigest().encode() != expected_oid: fail("tracked HEAD Rust blob hash is invalid")
        try: suppressed = detect_coverage_control(source)
        except SourcePolicyError as error: fail(f"tracked HEAD Rust source suppression scan failed: {error}")
        if suppressed: fail("tracked HEAD Rust source contains explicit coverage suppression")
        cursor = stop + 1
    if cursor != len(output): fail("tracked HEAD Rust blob batch is malformed")
def interval_size(intervals: list[tuple[int, int]]) -> int:
    total = end = 0
    for start, stop in sorted(intervals):
        if stop > end:
            total += stop - max(start, end + 1) + 1
            end = stop
    return total
def detailed_line_maps(regions: dict[str, list[tuple[int, int, bool]]]) -> dict[str, dict[int, bool]]:
    result: dict[str, dict[int, bool]] = {}
    observed = 0
    for path, items in regions.items():
        events: dict[int, list[int]] = {}
        for start, stop, covered in items:
            for point, direction in ((start, 1), (stop + 1, -1)):
                event = events.setdefault(point, [0, 0])
                event[0] += direction
                event[1] += direction if covered else 0
        active = positive = previous = 0
        line_map: dict[int, bool] = {}
        for point in sorted(events):
            if active:
                observed += point - previous
                if observed > MAX_DETAILED_LINES: fail("LLVM detailed line observation limit exceeded")
                for line in range(previous, point): line_map[line] = positive > 0
            active += events[point][0]; positive += events[point][1]; previous = point
        result[path] = line_map
    return result
def llvm_records(root: dict[str, Any], repo: Path, changed: dict[str, set[int]], head: str) -> dict[str, dict[str, Any]]:
    require_sha(head, "head")
    if root.get("type") != LLVM_TYPE or root.get("version") != LLVM_VERSION: fail("LLVM JSON type/version is not the reviewed export format")
    provenance = root.get("cargo_llvm_cov")
    if not isinstance(provenance, dict) or provenance.get("version") != "0.8.7": fail("LLVM JSON lacks cargo-llvm-cov 0.8.7 provenance")
    try:
        manifest = Path(provenance.get("manifest_path", "")).resolve(strict=True)
    except (OSError, TypeError):
        fail("LLVM JSON manifest provenance is invalid")
    if manifest != repo / "Cargo.toml": fail("LLVM JSON manifest provenance does not match repo-root")
    data_sets = root.get("data")
    if not isinstance(data_sets, list) or len(data_sets) != 1 or not isinstance(data_sets[0], dict): fail("LLVM JSON must contain one detailed data record")
    block = data_sets[0]
    functions, totals = block.get("functions"), block.get("totals")
    if not isinstance(functions, list) or not functions: fail("LLVM JSON detailed function records are missing")
    if not isinstance(totals, dict) or not isinstance(block.get("files"), list): fail("LLVM JSON detailed totals/files are missing")
    records: dict[str, dict[str, Any]] = {}
    source_limits: dict[str, list[int]] = {}
    source_line_total = 0
    def coordinate_exists(path: str, line: int, column: int) -> bool:
        nonlocal source_line_total
        if path not in source_limits:
            source = head_blob(repo, head, path)
            source_line_total += source.count(b"\n") + int(bool(source) and not source.endswith(b"\n"))
            if source_line_total > MAX_DETAILED_LINES: fail("LLVM coverage source line limit exceeded")
            lines = [] if not source else source.split(b"\n")
            if source.endswith(b"\n"): lines.pop()
            source_limits[path] = [len(item) + 1 for item in lines]
        return 0 < line <= len(source_limits[path]) and 0 < column <= source_limits[path][line - 1]
    for record in block["files"]:
        if not isinstance(record, dict): fail("LLVM JSON contains a malformed file record")
        path = normalize_source(record.get("filename"), repo)
        if path in records: fail("LLVM JSON contains duplicate normalized source records")
        if not isinstance(record.get("expansions"), list) or not isinstance(record.get("mcdc_records"), list): fail("LLVM JSON file is not a detailed export")
        segments, branches = record.get("segments"), record.get("branches")
        metrics = {kind: summary_counts(record, kind) for kind in METRIC_KINDS}
        line_counts, branch_counts = metrics["lines"], metrics["branches"]
        if not isinstance(segments, list) or (line_counts[0] and not segments) or not isinstance(branches, list):
            fail("LLVM JSON detailed segments/branches are missing")
        segment_details: dict[tuple[int, int], tuple[int, bool, bool]] = {}
        for segment in segments:
            if not isinstance(segment, list) or len(segment) != 6: fail("LLVM JSON segment shape or values are invalid")
            line, column, count, has_count, _entry, gap = segment
            if (
                any(isinstance(value, bool) or not isinstance(value, int) for value in segment[:3])
                or not coordinate_exists(path, line, column) or not 0 <= count <= MAX_LLVM_INTEGER
                or not all(isinstance(value, bool) for value in segment[3:])
            ):
                fail("LLVM JSON segment shape or values are invalid")
            if (line, column) in segment_details: fail("LLVM JSON contains duplicate segment coordinates")
            segment_details[(line, column)] = (count, has_count, gap)
        branch_observations: set[tuple[int, ...]] = set()
        branch_conditions: dict[tuple[int, ...], list[bool]] = {}
        for branch in branches:
            values = llvm_vector(branch, 9, "branch")
            if (
                not coordinate_exists(path, values[0], values[1]) or not coordinate_exists(path, values[2], values[3])
                or values[2] < values[0] or (values[2] == values[0] and values[3] < values[1]) or values[6:] != [0, 0, 4]
            ):
                fail("LLVM JSON branch shape or values are invalid")
            coordinate = (*values[:4], values[8])
            outcomes = branch_conditions.setdefault(coordinate, [False, False])
            for outcome in (4, 5):
                if values[outcome] > 0:
                    outcomes[outcome - 4] = True
                    branch_observations.add((*coordinate, outcome))
        if branch_counts[0] and (
            not branches or branch_counts[0] > len(branch_conditions) * 2 or branch_counts[1] > len(branch_observations)
        ):
            fail("LLVM detailed branches cannot support summary")
        branch_map: dict[int, bool] = {}
        for coordinate, outcomes in branch_conditions.items():
            branch_map[coordinate[0]] = branch_map.get(coordinate[0], True) and all(outcomes)
        positive_segment = any(item[2] > 0 and item[3] and not item[5] for item in segments)
        records[path] = {"lines": line_counts, "branches": branch_counts, "metrics": metrics, "branch_detail": branch_conditions, "branch_map": branch_map, "positive_segment": positive_segment, "segment_details": segment_details}
    if not records: fail("LLVM JSON has no file records")
    detailed_lines: set[str] = set()
    detailed_branches: set[str] = set()
    positive_lines: set[str] = set()
    function_branch_detail: dict[str, dict[tuple[int, ...], list[bool]]] = {}
    line_regions: dict[str, list[tuple[int, int, bool]]] = {}
    function_groups: dict[str, dict[tuple[int, ...], list[tuple[int, ...]]]] = {}
    for function in functions:
        if not isinstance(function, dict) or not isinstance(function.get("name"), str) or not function["name"]: fail("LLVM JSON function record is invalid")
        function_count = integer(function.get("count"), "LLVM function count")
        filenames, regions, branches = function.get("filenames"), function.get("regions"), function.get("branches")
        if (
            not isinstance(filenames, list) or len(filenames) != 1 or not isinstance(regions, list) or not regions
            or not isinstance(branches, list) or not isinstance(function.get("mcdc_records"), list)
        ):
            fail("LLVM JSON function details are missing")
        path = function_source(filenames[0], repo)
        parsed_regions: list[list[int]] = []
        for raw_region in regions:
            region = llvm_vector(raw_region, 8, "function region")
            if (
                region[0] <= 0 or region[1] <= 0 or region[3] <= 0 or region[2] < region[0]
                or (region[2] == region[0] and region[3] < region[1]) or region[5:] != [0, 0, 0]
            ):
                fail("LLVM JSON function region is invalid")
            if path is not None and (not coordinate_exists(path, region[0], region[1]) or not coordinate_exists(path, region[2], region[3])):
                fail("LLVM function region exceeds source lines or columns")
            parsed_regions.append(region)
        if function_count != parsed_regions[0][4]: fail("LLVM function count disagrees with its primary region")
        if not function_count and any(item[4] for item in parsed_regions): fail("zero-count LLVM function has a covered region")
        if path in records:
            first = parsed_regions[0]
            segments = records[path]["segment_details"]
            if (first[2], first[3]) not in segments:
                fail("LLVM function anchor lacks file segment support")
            for item in parsed_regions:
                segment = segments.get((item[0], item[1]))
                if segment is None or not segment[1] or segment[2]: fail("LLVM function region lacks counted file segment support")
            intervals = [(item[0], item[2]) for item in parsed_regions]
            positive = [(item[0], item[2]) for item in parsed_regions if item[4] > 0]
            instance = (function_count > 0, len(parsed_regions), len(positive), interval_size(intervals), interval_size(positive))
            function_groups.setdefault(path, {}).setdefault((first[0], first[1]), []).append(instance)
            line_regions.setdefault(path, []).extend((item[0], item[2], item[4] > 0) for item in parsed_regions)
            detailed_lines.add(path)
            if positive: positive_lines.add(path)
        for raw_branch in branches:
            branch = llvm_vector(raw_branch, 9, "function branch")
            if (
                branch[0] <= 0 or branch[1] <= 0 or branch[3] <= 0 or branch[2] < branch[0]
                or (branch[2] == branch[0] and branch[3] < branch[1]) or branch[6:] != [0, 0, 4]
                or not function_count and (branch[4] > 0 or branch[5] > 0)
            ):
                fail("LLVM JSON function branch is invalid")
            if path is not None and (not coordinate_exists(path, branch[0], branch[1]) or not coordinate_exists(path, branch[2], branch[3])):
                fail("LLVM function branch exceeds source lines or columns")
            if path in records:
                detailed_branches.add(path)
                coordinate = (*branch[:4], branch[8])
                outcomes = function_branch_detail.setdefault(path, {}).setdefault(coordinate, [False, False])
                for outcome in (4, 5):
                    if branch[outcome] > 0: outcomes[outcome - 4] = True
    line_details = detailed_line_maps(line_regions)
    for path, record in records.items():
        if record["lines"][0] and path not in detailed_lines:
            fail("LLVM file summary lacks detailed function regions")
        if record["lines"][1] and (not record["positive_segment"] or path not in positive_lines): fail("LLVM covered summary lacks positive detailed regions")
        groups = function_groups.get(path, {})
        observed = {
            "instantiations": (sum(len(items) for items in groups.values()), sum(sum(item[0] for item in items) for items in groups.values())),
            "functions": (len(groups), sum(any(item[0] for item in items) for items in groups.values())),
            "regions": (sum(max(item[1] for item in items) for items in groups.values()), sum(max(item[2] for item in items) for items in groups.values())),
            "lines": (sum(max(item[3] for item in items) for items in groups.values()), sum(max(item[4] for item in items) for items in groups.values())),
        }
        for kind, counts in observed.items():
            if counts != record["metrics"][kind]: fail(f"LLVM detailed functions disagree with {kind} summary")
        if record["branches"][0] and path not in detailed_branches:
            fail("LLVM branch summary lacks detailed function branches")
        if record["branches"][0] % 2: fail("LLVM branch summary count must be even")
        function_detail = function_branch_detail.get(path, {})
        if any(record["branch_detail"].get(key) != value for key, value in function_detail.items()): fail("LLVM file/function branch details disagree")
        if any(any(value) for key, value in record["branch_detail"].items() if key not in function_detail): fail("LLVM file/function branch details disagree")
        record["function_branch_detail"] = function_detail
        record["line_map"] = line_details.get(path, {})
        record["changed"] = {line: record["line_map"][line] for line in changed.get(path, ()) if line in record["line_map"]}
    for kind in METRIC_KINDS:
        aggregate = tuple(sum(record["metrics"][kind][index] for record in records.values()) for index in (0, 1))
        if metric_counts(totals.get(kind), f"total {kind}", kind in ("regions", "branches", "mcdc")) != aggregate:
            fail(f"LLVM total {kind} summary is inconsistent")
    return records
def xml_rate(element: ET.Element, attribute: str, counts: tuple[int, int], label: str) -> None:
    try:
        value = float(element.attrib[attribute])
    except (KeyError, ValueError):
        fail(f"Cobertura XML {label} rate is invalid")
    expected = 0.0 if counts[0] == 0 else counts[1] / counts[0]
    if not math.isfinite(value) or abs(value - expected) > 1e-12:
        fail(f"Cobertura XML {label} rate is inconsistent")
def cobertura_records(data: bytes, repo: Path) -> dict[str, dict[str, Any]]:
    if not data.strip(): fail("Cobertura XML is empty")
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        fail("Cobertura XML is malformed")
    if root.tag != "coverage" or root.get("version") != COBERTURA_VERSION: fail("Cobertura XML type/version is not the reviewed export format")
    sources = root.findall("./sources/source")
    if len(sources) != 1 or not sources[0].text or Path(sources[0].text).resolve(strict=False) != repo: fail("Cobertura XML source provenance does not match repo-root")
    packages = root.findall("./packages/package")
    if not packages: fail("Cobertura XML has no class records")
    records: dict[str, dict[str, Any]] = {}
    package_totals: list[tuple[tuple[int, int], tuple[int, int]]] = []
    for package in packages:
        classes = package.findall("./classes/class")
        if not classes: fail("Cobertura XML package has no class records")
        p_lines, p_branches = [0, 0], [0, 0]
        for record in classes:
            path = normalize_source(record.get("filename"), repo)
            if path in records: fail("Cobertura XML contains duplicate normalized source records")
            lines: dict[int, bool] = {}
            branch_total = branch_covered = 0
            branch_map: dict[int, bool] = {}
            branch_detail: dict[int, tuple[int, int]] = {}
            for line in record.findall("./lines/line"):
                try:
                    number, hits = int(line.attrib["number"]), int(line.attrib["hits"])
                except (KeyError, ValueError):
                    fail("Cobertura XML contains a malformed line record")
                if number <= 0 or hits < 0 or number in lines: fail("Cobertura XML contains an invalid or duplicate line observation")
                lines[number] = hits > 0
                branch = line.get("branch", "false")
                if branch not in ("true", "false"): fail("Cobertura XML branch marker is invalid")
                if branch == "true":
                    match = CONDITION_RE.fullmatch(line.get("condition-coverage", ""))
                    if match is None: fail("Cobertura XML branch condition observation is missing")
                    percent, item_covered, item_total = map(int, match.groups())
                    if item_total <= 0 or item_covered > item_total or percent != item_covered * 100 // item_total:
                        fail("Cobertura XML branch condition counts are invalid")
                    branch_total += item_total
                    branch_covered += item_covered
                    branch_map[number] = item_covered == item_total
                    branch_detail[number] = (item_total, item_covered)
            line_counts, branch_counts = (len(lines), sum(lines.values())), (branch_total, branch_covered)
            xml_rate(record, "line-rate", line_counts, "class line")
            xml_rate(record, "branch-rate", branch_counts, "class branch")
            for index in (0, 1):
                p_lines[index] += line_counts[index]
                p_branches[index] += branch_counts[index]
            records[path] = {"line_map": lines, "branch_map": branch_map, "branch_detail": branch_detail, "lines": line_counts, "branches": branch_counts}
        p_counts = (tuple(p_lines), tuple(p_branches))
        xml_rate(package, "line-rate", p_counts[0], "package line")
        xml_rate(package, "branch-rate", p_counts[1], "package branch")
        package_totals.append(p_counts)
    root_lines = tuple(sum(item[0][index] for item in package_totals) for index in (0, 1))
    root_branches = tuple(sum(item[1][index] for item in package_totals) for index in (0, 1))
    try:
        declared = (
            (int(root.attrib["lines-valid"]), int(root.attrib["lines-covered"])),
            (int(root.attrib["branches-valid"]), int(root.attrib["branches-covered"])),
        )
    except (KeyError, ValueError):
        fail("Cobertura XML root aggregate is invalid")
    if declared != (root_lines, root_branches): fail("Cobertura XML root aggregate is inconsistent")
    xml_rate(root, "line-rate", root_lines, "root line")
    xml_rate(root, "branch-rate", root_branches, "root branch")
    return records
def crosscheck_records(llvm: dict[str, dict[str, Any]], cobertura: dict[str, dict[str, Any]]) -> None:
    if set(llvm) != set(cobertura): fail("LLVM and Cobertura source record sets disagree")
    for path in llvm:
        raw_lines, xml_lines = llvm[path]["line_map"], cobertura[path]["line_map"]
        raw_branches, xml_branches = llvm[path]["branch_map"], cobertura[path]["branch_map"]
        if any(xml_lines.get(line) is not covered for line, covered in raw_lines.items()):
            fail("LLVM and Cobertura detailed line coverage disagrees")
        for line in set(xml_lines) - set(raw_lines):
            if xml_lines[line] or line not in raw_branches or line not in xml_branches or xml_branches[line] is not raw_branches[line] or not any(key[0] == line for key in llvm[path]["function_branch_detail"]):
                fail("Cobertura has an unsupported extra line observation")
        if any(raw_branches.get(line) is not covered for line, covered in xml_branches.items()):
            fail("LLVM and Cobertura detailed branch coverage disagrees")
        if any(covered for line, covered in raw_branches.items() if line not in xml_branches):
            fail("Cobertura is missing a covered LLVM branch observation")
def coverage_metrics(repo: Path, diff: bytes, raw: bytes, xml: bytes, head: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    added_bodies: dict[str, list[bytes]] = {}
    changed_paths: set[str] = set()
    added = parse_changed_lines(diff, repo, added_bodies=added_bodies, changed_paths=changed_paths)
    reject_coverage_affecting_changes(changed_paths)
    enforce_tracked_head_coverage_policy(repo, head)
    llvm = llvm_records(strict_json(raw), repo, added, head)
    cobertura = cobertura_records(xml, repo)
    crosscheck_records(llvm, cobertura)
    policies = {}
    for path in set(added) | {item[1] for item in CRITICAL_MODULES}:
        try:
            policies[path] = analyze_rust_source(head_blob(repo, head, path))
        except SourcePolicyError as error:
            fail(f"Rust source policy cannot be classified: {error}")
    if any(policies[path].coverage_control for path in added):
        fail("changed production Rust source contains explicit coverage suppression")
    total = covered = 0
    files: list[dict[str, Any]] = []
    for path in sorted(added):
        if path not in llvm:
            if not added[path]: fail("a deletion-only production Rust path is missing from coverage artifacts")
            if not reviewed_declaration_only_change(path, added[path], added_bodies.get(path, [])):
                fail("a changed production Rust path is missing from coverage artifacts")
            files.append({"path": path, "added": len(added[path]), "executable": 0, "covered": 0})
            continue
        changed_map = {line: cobertura[path]["line_map"][line] for line in added[path] if line in cobertura[path]["line_map"] and line not in policies[path].test_only_lines}
        file_covered = sum(changed_map.values())
        executable = sorted(changed_map)
        total += len(executable)
        covered += file_covered
        files.append({"path": path, "added": len(added[path]), "executable": len(executable), "covered": file_covered})
    if total == 0: fail("changed production lines have no detailed executable observations")
    if covered * 100 < total * 80: fail("changed executable line coverage is below 80%")
    critical: list[dict[str, Any]] = []
    for label, path in CRITICAL_MODULES:
        if path not in llvm or path not in cobertura: fail(f"critical module is missing: {label}")
        excluded = policies[path].test_only_lines
        llvm_lines = {line: value for line, value in llvm[path]["line_map"].items() if line not in excluded}
        xml_lines = {line: value for line, value in cobertura[path]["line_map"].items() if line not in excluded}
        llvm_branch_detail = {key: value for key, value in llvm[path]["function_branch_detail"].items() if key[0] not in excluded}
        xml_branch_detail = {line: value for line, value in cobertura[path]["branch_detail"].items() if line not in excluded}
        llvm_branches: dict[int, bool] = {}
        for key, outcomes in llvm_branch_detail.items():
            llvm_branches[key[0]] = llvm_branches.get(key[0], True) and all(outcomes)
        xml_branches = {line: value for line, value in cobertura[path]["branch_map"].items() if line not in excluded}
        llvm_counts = {"lines": (len(llvm_lines), sum(llvm_lines.values())), "branches": (len(llvm_branch_detail) * 2, sum(sum(value) for value in llvm_branch_detail.values()))}
        xml_counts = {"lines": (len(xml_lines), sum(xml_lines.values())), "branches": tuple(sum(value[index] for value in xml_branch_detail.values()) for index in (0, 1))}
        details = (llvm_lines, llvm_branches, xml_lines, xml_branches)
        if any(not item for item in details) or any(counts[0] == 0 for counts in (*llvm_counts.values(), *xml_counts.values())):
            fail(f"critical module has zero detailed observations: {label}")
        if any(counts[0] != counts[1] for counts in (*llvm_counts.values(), *xml_counts.values())) or any(not all(item.values()) for item in details):
            fail(f"critical module is not 100% covered: {label}")
        critical.append({
            "label": label, "path": path,
            "llvm": {kind: {"total": counts[0], "covered": counts[1]} for kind, counts in llvm_counts.items()},
            "cobertura": {kind: {"total": counts[0], "covered": counts[1]} for kind, counts in xml_counts.items()},
        })
    return {"minimum_percent": 80, "total": total, "covered": covered, "files": files}, critical
