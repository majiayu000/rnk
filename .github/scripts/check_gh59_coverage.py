#!/usr/bin/env python3
"""Collect, produce, and independently validate GH-59 coverage evidence."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from gh59_coverage_lib import (
    CHECKER_VERSION,
    COLLECTION_SCHEMA,
    CONDITION_RE,
    EVIDENCE_SCHEMA,
    LLVM_COV_VERSION,
    MAX_COVERAGE_BYTES,
    MAX_DOCUMENT_BYTES,
    METRIC_KINDS,
    CheckError,
    canonical_json,
    coverage_metrics,
    fail,
    integer,
    parse_changed_lines,
    read_regular,
    repository_snapshot,
    require_distinct,
    safe_path,
    sha256,
    strict_json,
)


TOOL_ENV_REMOVALS = {
    "CARGO", "CARGO_INCREMENTAL", "DYLD_INSERT_LIBRARIES", "LD_PRELOAD", "RUSTC",
    "RUSTC_BOOTSTRAP", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_WRAPPER", "RUSTDOC",
    "RUSTDOCFLAGS", "RUSTFLAGS", "RUSTUP_TOOLCHAIN",
}
TOOL_ENV_PREFIXES = (
    "CARGO_ALIAS_", "CARGO_BUILD_", "CARGO_ENCODED_", "CARGO_LLVM_",
    "CARGO_PROFILE_", "CARGO_TARGET_", "DYLD_", "LD_", "LLVM_", "RUSTC_", "__CARGO_LLVM_COV",
)


def tool_environment(cargo: str | None = None, rustc: str | None = None) -> dict[str, str]:
    environment = os.environ.copy()
    for key in list(environment):
        if key in TOOL_ENV_REMOVALS or key.startswith(TOOL_ENV_PREFIXES):
            environment.pop(key)
    environment.update({"LC_ALL": "C", "LANG": "C", "CARGO_TERM_COLOR": "never", "RUSTUP_TOOLCHAIN": "nightly"})
    if cargo is not None: environment["CARGO"] = cargo
    if rustc is not None: environment["RUSTC"] = rustc
    return environment


def tool_paths(tools: dict[str, Any] | None) -> tuple[str | None, str | None]:
    if tools is None: return None, None
    return tools["cargo"]["executable"]["launcher"], tools["rustc"]["executable"]["launcher"]


def run_tool(command: list[str], repo: Path, label: str, capture: bool = False, limit: int = 16_384, tools: dict[str, Any] | None = None) -> str:
    output = subprocess.PIPE if capture else subprocess.DEVNULL
    cargo, rustc = tool_paths(tools)
    try:
        result = subprocess.run(command, cwd=repo, env=tool_environment(cargo, rustc), stdout=output, stderr=subprocess.DEVNULL, check=False)
    except OSError as error:
        fail(f"{label} execution failed: {error.__class__.__name__}")
    if result.returncode != 0: fail(f"{label} command failed")
    if not capture: return ""
    try:
        value = result.stdout.decode().strip()
    except UnicodeDecodeError:
        fail(f"{label} output is not UTF-8")
    if not value or len(value.encode()) > limit: fail(f"{label} output is invalid")
    return value


def reject_cargo_config(repo: Path) -> None:
    candidates = [parent / ".cargo" / name for parent in (repo, *repo.parents) for name in ("config.toml", "config")]
    raw_home = os.environ.get("CARGO_HOME")
    cargo_home = Path(raw_home) if raw_home is not None else Path.home() / ".cargo"
    if not cargo_home.is_absolute(): fail("CARGO_HOME must be absolute")
    candidates.extend(cargo_home.resolve(strict=False) / name for name in ("config.toml", "config"))
    if any(os.path.lexists(path) for path in candidates):
        fail("Cargo configuration is forbidden during reviewed coverage collection")


def executable_record(path: str, label: str, resolution: list[str] | None = None) -> dict[str, Any]:
    candidate = Path(path)
    if not candidate.is_absolute(): fail(f"{label} executable is unavailable")
    try:
        target = candidate.resolve(strict=True)
    except OSError:
        fail(f"{label} executable cannot be resolved")
    _data, identity = read_regular(target, f"{label} executable", MAX_COVERAGE_BYTES)
    record = {"launcher": str(candidate), "target": str(target), "size": identity[2], "sha256": identity[4]}
    if resolution is not None: record["resolution_command"] = resolution
    return record


def version_provenance(value: str, label: str) -> dict[str, str]:
    lines = value.splitlines()
    if not lines or not lines[0].startswith(f"{label} "): fail("the pinned nightly Cargo/Rust compiler is unavailable")
    fields = {key: item for line in lines[1:] if ": " in line for key, item in [line.split(": ", 1)]}
    if any(not fields.get(key) for key in ("release", "commit-date", "host")) or not fields["release"].endswith("-nightly"):
        fail("the pinned nightly Cargo/Rust compiler is unavailable")
    return {key: fields[key] for key in ("release", "commit-date", "host")}


def toolchain_provenance_matches(cargo: dict[str, str], rustc: dict[str, str]) -> bool:
    return all(cargo[key] == rustc[key] for key in ("release", "host"))


def toolchain(repo: Path) -> dict[str, Any]:
    reject_cargo_config(repo)
    rustup = shutil.which("rustup")
    plugin = shutil.which("cargo-llvm-cov")
    if not rustup or not Path(rustup).is_absolute(): fail("rustup executable is unavailable")
    if not plugin or not Path(plugin).is_absolute(): fail("cargo-llvm-cov executable is unavailable")
    executable: dict[str, dict[str, Any]] = {"cargo_llvm_cov": executable_record(plugin, "cargo-llvm-cov")}
    for name in ("cargo", "rustc"):
        command = [rustup, "which", name, "--toolchain", "nightly"]
        executable[name] = executable_record(run_tool(command, repo, f"nightly {name} resolution", True), name, command)
    if Path(executable["cargo"]["target"]).parent != Path(executable["rustc"]["target"]).parent:
        fail("nightly Cargo and Rust compiler do not share one toolchain")
    commands = {
        "cargo_llvm_cov": [executable["cargo_llvm_cov"]["launcher"], "llvm-cov", "--version"],
        "cargo": [executable["cargo"]["launcher"], "--version", "--verbose"],
        "rustc": [executable["rustc"]["launcher"], "--version", "--verbose"],
    }
    partial = {key: {"command": commands[key], "executable": executable[key]} for key in commands}
    versions = {key: run_tool(command, repo, key.replace("_", "-"), True, tools=partial) for key, command in commands.items()}
    if versions["cargo_llvm_cov"] != LLVM_COV_VERSION: fail("cargo-llvm-cov version must be exactly 0.8.7")
    provenance = {key: version_provenance(versions[key], key) for key in ("cargo", "rustc")}
    if not toolchain_provenance_matches(provenance["cargo"], provenance["rustc"]): fail("nightly Cargo/Rust provenance does not match")
    return {key: {**partial[key], "version": versions[key], **({"provenance": provenance[key]} if key in provenance else {})} for key in commands}


def workspace_metadata(repo: Path, tools: dict[str, Any]) -> dict[str, Any]:
    cargo = tools["cargo"]["executable"]["launcher"]
    command = [cargo, "metadata", "--format-version", "1", "--no-deps", "--locked", "--manifest-path", str(repo / "Cargo.toml")]
    value = strict_json(run_tool(command, repo, "Cargo metadata", True, MAX_DOCUMENT_BYTES, tools).encode(), "Cargo metadata")
    packages, members = value.get("packages"), value.get("workspace_members")
    if value.get("version") != 1 or not isinstance(packages, list) or not isinstance(members, list) or not members: fail("Cargo metadata schema is invalid")
    by_id: dict[str, str] = {}
    for package in packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str) or not isinstance(package.get("name"), str): fail("Cargo metadata package is invalid")
        manifest = Path(package.get("manifest_path", "")).resolve(strict=False)
        try: manifest.relative_to(repo)
        except ValueError: fail("Cargo workspace package escapes repo-root")
        if package["id"] in by_id or not package["name"]: fail("Cargo metadata package identity is invalid")
        by_id[package["id"]] = package["name"]
    if any(not isinstance(item, str) or item not in by_id for item in members): fail("Cargo workspace member is invalid")
    names = sorted(by_id[item] for item in members)
    if len(names) != len(set(names)): fail("Cargo workspace package names are not unique")
    return {"command": command, "sha256": sha256(canonical_json(value)), "packages": names}


def collection_commands(repo: Path, llvm: Path, xml: Path, plugin: str, packages: list[str]) -> list[list[str]]:
    manifest = str(repo / "Cargo.toml")
    package_args = [item for name in packages for item in ("--package", name)]
    return [
        [plugin, "llvm-cov", "clean", "--workspace", "--manifest-path", manifest],
        [plugin, "llvm-cov", "--no-cfg-coverage", "--no-cfg-coverage-nightly", "--workspace", "--all-targets", "--all-features", "--locked", "--branch", "--color", "never", "--manifest-path", manifest, "--json", "--output-path", str(llvm)],
        [plugin, "llvm-cov", "report", *package_args, "--cobertura", "--color", "never", "--manifest-path", manifest, "--output-path", str(xml)],
    ]


def atomic_write(path: Path, data: bytes, label: str, callback: Any = None) -> None:
    safe_path(path, label)
    if not path.parent.is_dir(): fail(f"{label} parent directory does not exist")
    if os.path.lexists(path): fail(f"{label} must not preexist")
    created: tuple[int, int] | None = None
    try:
        descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        try:
            with os.fdopen(descriptor, "wb") as output:
                output.write(data)
                output.flush()
                os.fsync(output.fileno())
                created = (os.fstat(output.fileno()).st_dev, os.fstat(output.fileno()).st_ino)
            if callback: callback()
            os.link(temporary, path)
            Path(temporary).unlink()
            if read_regular(path, label, MAX_DOCUMENT_BYTES)[0] != data: fail(f"{label} changed while publishing")
            if callback: callback()
            final, identity = read_regular(path, label, MAX_DOCUMENT_BYTES)
            if final != data or identity[:2] != created: fail(f"{label} changed after final check")
        except BaseException:
            Path(temporary).unlink(missing_ok=True)
            try:
                info = path.lstat()
                if created == (info.st_dev, info.st_ino): path.unlink()
            except FileNotFoundError:
                created = None
            raise
    except OSError as error:
        fail(f"{label} could not be written: {error.__class__.__name__}")


def receipt_value(snapshot: dict[str, Any], tools: dict[str, Any], workspace: dict[str, Any], commands: list[list[str]], started: int, finished: int, repo: Path, manifest: bytes, llvm: tuple[Path, bytes, tuple[Any, ...]], xml: tuple[Path, bytes, tuple[Any, ...]]) -> dict[str, Any]:
    return {
        "schema": COLLECTION_SCHEMA, "status": "complete",
        "collector": {"path": ".github/scripts/check_gh59_coverage.py", "version": CHECKER_VERSION},
        "git": snapshot, "toolchain": tools,
        "collection": {"started_ns": started, "finished_ns": finished, "working_directory": str(repo), "manifest_path": "Cargo.toml", "manifest_sha256": sha256(manifest), "workspace": workspace, "commands": commands},
        "artifacts": {
            "llvm_json": {"name": llvm[0].name, "size": llvm[2][2], "sha256": llvm[2][4]},
            "cobertura": {"name": xml[0].name, "size": xml[2][2], "sha256": xml[2][4]},
        },
    }


def snapshot_from_args(args: argparse.Namespace) -> tuple[Path, dict[str, Any], bytes]:
    return repository_snapshot(args.repo_root, args.head, args.base, args.merge_base, args.trusted_base_ref)


def collect(args: argparse.Namespace) -> None:
    repo, snapshot, diff = snapshot_from_args(args)
    manifest = repo / "Cargo.toml"
    manifest_bytes, manifest_id = read_regular(manifest, "Cargo manifest", MAX_DOCUMENT_BYTES)
    output = safe_path(args.output_dir, "output-dir")
    if os.path.lexists(output): fail("output-dir must not preexist")
    if not output.parent.is_dir(): fail("output-dir parent must be an existing directory")
    tools = toolchain(repo)
    workspace = workspace_metadata(repo, tools)
    started = time.time_ns()
    try:
        os.mkdir(output, 0o700)
    except OSError as error:
        fail(f"output-dir could not be created: {error.__class__.__name__}")
    llvm_path, xml_path = output / "llvm-cov.json", output / "cobertura.xml"
    commands = collection_commands(repo, llvm_path, xml_path, tools["cargo_llvm_cov"]["executable"]["launcher"], workspace["packages"])
    for index, command in enumerate(commands):
        run_tool(command, repo, f"collection step {index + 1}", tools=tools)
    repo_after, snapshot_after, diff_after = snapshot_from_args(args)
    if repo_after != repo or snapshot_after != snapshot or diff_after != diff: fail("repository changed during coverage collection")
    require_distinct([("LLVM JSON", llvm_path), ("Cobertura XML", xml_path)])
    llvm = (llvm_path, *read_regular(llvm_path, "LLVM JSON", MAX_COVERAGE_BYTES))
    xml = (xml_path, *read_regular(xml_path, "Cobertura XML", MAX_COVERAGE_BYTES))
    coverage_metrics(repo, diff, llvm[1], xml[1], snapshot["head_sha"])
    if read_regular(manifest, "Cargo manifest", MAX_DOCUMENT_BYTES)[1] != manifest_id: fail("Cargo manifest changed during collection")
    finished = time.time_ns()
    if any(not started <= int(item[2][3]) <= finished for item in (llvm, xml)): fail("coverage artifact timestamp is outside the collection window")
    value = receipt_value(snapshot, tools, workspace, commands, started, finished, repo, manifest_bytes, llvm, xml)
    def stable() -> None:
        if snapshot_from_args(args)[1:] != (snapshot, diff): fail("repository changed during coverage collection")
        for path, label, limit, identity in ((manifest, "Cargo manifest", MAX_DOCUMENT_BYTES, manifest_id), (llvm_path, "LLVM JSON", MAX_COVERAGE_BYTES, llvm[2]), (xml_path, "Cobertura XML", MAX_COVERAGE_BYTES, xml[2])):
            if read_regular(path, label, limit)[1] != identity: fail(f"{label} changed during collection")
        for item in tools.values():
            executable = item["executable"]
            if str(Path(executable["launcher"]).resolve(strict=True)) != executable["target"] or read_regular(Path(executable["target"]), "tool executable", MAX_COVERAGE_BYTES)[1][2::2] != (executable["size"], executable["sha256"]): fail("tool executable changed during collection")
    atomic_write(output / "collection-receipt.json", canonical_json(value), "collection receipt", stable)


def validate_receipt(args: argparse.Namespace) -> dict[str, Any]:
    llvm_path, xml_path, receipt_path = map(safe_path, (args.llvm_json, args.cobertura, args.receipt), ("LLVM JSON", "Cobertura XML", "collection receipt"))
    if (llvm_path.name, xml_path.name, receipt_path.name) != ("llvm-cov.json", "cobertura.xml", "collection-receipt.json") or not llvm_path.parent == xml_path.parent == receipt_path.parent:
        fail("collection artifacts must use fixed names in one directory")
    require_distinct([("LLVM JSON", llvm_path), ("Cobertura XML", xml_path), ("collection receipt", receipt_path)])
    receipt_bytes, receipt_id = read_regular(receipt_path, "collection receipt", MAX_DOCUMENT_BYTES)
    document = strict_json(receipt_bytes, "collection receipt")
    collection_data = document.get("collection")
    if not isinstance(collection_data, dict): fail("collection receipt lacks collection metadata")
    started = integer(collection_data.get("started_ns"), "collection start")
    finished = integer(collection_data.get("finished_ns"), "collection finish")
    if started <= 0 or started > finished: fail("collection receipt time window is invalid")
    repo, snapshot, diff = snapshot_from_args(args)
    llvm = (llvm_path, *read_regular(llvm_path, "LLVM JSON", MAX_COVERAGE_BYTES))
    xml = (xml_path, *read_regular(xml_path, "Cobertura XML", MAX_COVERAGE_BYTES))
    if any(not started <= int(item[2][3]) <= finished for item in (llvm, xml)): fail("coverage artifact timestamp is outside the collection window")
    manifest_path = repo / "Cargo.toml"
    manifest, manifest_id = read_regular(manifest_path, "Cargo manifest", MAX_DOCUMENT_BYTES)
    tools = toolchain(repo)
    workspace = workspace_metadata(repo, tools)
    commands = collection_commands(repo, llvm_path, xml_path, tools["cargo_llvm_cov"]["executable"]["launcher"], workspace["packages"])
    expected = receipt_value(snapshot, tools, workspace, commands, started, finished, repo, manifest, llvm, xml)
    if receipt_bytes != canonical_json(expected): fail("collection receipt does not match independently recomputed provenance")
    changed, critical = coverage_metrics(repo, diff, llvm[1], xml[1], snapshot["head_sha"])
    _repo, snapshot_after, diff_after = snapshot_from_args(args)
    if snapshot_after != snapshot or diff_after != diff: fail("repository changed during receipt validation")
    for path, label, limit, identity in ((manifest_path, "Cargo manifest", MAX_DOCUMENT_BYTES, manifest_id), (llvm_path, "LLVM JSON", MAX_COVERAGE_BYTES, llvm[2]), (xml_path, "Cobertura XML", MAX_COVERAGE_BYTES, xml[2]), (receipt_path, "collection receipt", MAX_DOCUMENT_BYTES, receipt_id)):
        if read_regular(path, label, limit)[1] != identity: fail(f"{label} changed during validation")
    return {"snapshot": snapshot, "llvm": llvm, "xml": xml, "receipt": (receipt_path, receipt_bytes, receipt_id), "changed": changed, "critical": critical}


def build_evidence(context: dict[str, Any]) -> dict[str, Any]:
    snapshot = context["snapshot"]
    llvm, xml, receipt = context["llvm"], context["xml"], context["receipt"]
    return {
        "schema": EVIDENCE_SCHEMA, "decision": "allowed",
        "head_sha": snapshot["head_sha"], "base_sha": snapshot["base_sha"],
        "merge_base_sha": snapshot["merge_base_sha"], "generated_at": snapshot["commit_timestamp"],
        "provenance": {"checker_version": CHECKER_VERSION, "trusted_base_ref": snapshot["trusted_base_ref"], "diff_command": snapshot["diff_command"], "diff_sha256": snapshot["diff_sha256"], "collection_receipt_sha256": sha256(receipt[1]), "llvm_json_sha256": sha256(llvm[1]), "cobertura_sha256": sha256(xml[1])},
        "changed_executable": context["changed"], "critical": context["critical"],
    }


def final_check(args: argparse.Namespace, context: dict[str, Any]) -> None:
    if snapshot_from_args(args)[1] != context["snapshot"]: fail("repository changed before success")
    for path, _data, identity in (context["llvm"], context["xml"], context["receipt"]):
        limit = MAX_DOCUMENT_BYTES if path.name == "collection-receipt.json" else MAX_COVERAGE_BYTES
        if read_regular(path, path.name, limit)[1] != identity: fail("collection input changed before success")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="subcommand", required=True)
    def identity(command: argparse.ArgumentParser) -> None:
        for name in ("repo-root", "head", "base", "merge-base", "trusted-base-ref"):
            command.add_argument(f"--{name}", required=True)
    collect_parser = commands.add_parser("collect")
    identity(collect_parser)
    collect_parser.add_argument("--output-dir", required=True, type=Path)
    for name in ("produce", "validate"):
        command = commands.add_parser(name)
        identity(command)
        for option in ("llvm-json", "cobertura", "receipt", "evidence"):
            command.add_argument(f"--{option}", required=True, type=Path)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.subcommand == "collect":
            collect(args)
            print("GH59 coverage collection complete")
            return 0
        context = validate_receipt(args)
        evidence_path = safe_path(args.evidence, "coverage evidence")
        require_distinct([("LLVM JSON", context["llvm"][0]), ("Cobertura XML", context["xml"][0]), ("collection receipt", context["receipt"][0]), ("coverage evidence", evidence_path)], args.subcommand == "produce")
        expected = canonical_json(build_evidence(context))
        if args.subcommand == "produce":
            if os.path.lexists(evidence_path): fail("coverage evidence must not preexist")
            atomic_write(evidence_path, expected, "coverage evidence", lambda: final_check(args, context))
            print("GH59 coverage evidence produced: decision=allowed")
        else:
            actual, identity = read_regular(evidence_path, "coverage evidence", MAX_DOCUMENT_BYTES)
            if actual != expected: fail("evidence bytes do not match independently recomputed evidence")
            final_check(args, context)
            if read_regular(evidence_path, "coverage evidence", MAX_DOCUMENT_BYTES)[1] != identity: fail("coverage evidence changed during validation")
            final_check(args, context)
            print("GH59 coverage evidence validated: decision=allowed")
        return 0
    except CheckError as error:
        print(f"GH59 coverage check failed: {error}", file=sys.stderr)
        return 1
    except Exception as error:
        print(f"GH59 coverage check failed: unexpected {error.__class__.__name__}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
