#!/usr/bin/env python3
"""Isolated fixture tests for the GH-59 coverage checker."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path


SCRIPT = Path(__file__).with_name("check_gh59_coverage.py")
sys.path.insert(0, str(SCRIPT.parent))
import check_gh59_coverage as checker  # noqa: E402


CRITICAL = (
    "src/reconciler/identity.rs",
    "src/reconciler/plan.rs",
    "src/layout/engine/incremental_order.rs",
    "src/layout/engine/incremental.rs",
)
FAKE_CARGO = r'''#!/usr/bin/env python3
import os, shutil, subprocess, sys
args = sys.argv[1:]
if any(key in os.environ for key in ("CARGO_LLVM_COV_TARGET_DIR", "__CARGO_LLVM_COV_RUSTC_WRAPPER", "LLVM_COV_FLAGS")):
    raise SystemExit(18)
if args == ["+nightly", "llvm-cov", "--version"]:
    print(os.environ.get("GH59_FAKE_TOOL_VERSION", "cargo-llvm-cov 0.8.7")); raise SystemExit(0)
if args in (["+nightly", "--version"], ["--version", "--verbose"]):
    print("cargo 1.95.0-nightly (fixture)\nrelease: 1.95.0-nightly\ncommit-date: 2026-02-11\nhost: fixture-target"); raise SystemExit(0)
if args[:2] == ["+nightly", "metadata"] or args[:1] == ["metadata"]:
    print(open(os.environ["GH59_FAKE_METADATA"]).read()); raise SystemExit(0)
if "clean" in args:
    raise SystemExit(0)
output = args[args.index("--output-path") + 1]
template = os.environ["GH59_FAKE_LLVM"] if "--json" in args else os.environ["GH59_FAKE_XML"]
if os.environ.get("GH59_FAKE_SYMLINK") == "1" and "--json" in args:
    os.symlink(template, output)
else:
    shutil.copyfile(template, output)
if "--cobertura" in args:
    if "--package" not in args or args[args.index("--package") + 1] != "fixture":
        raise SystemExit(19)
    repo = os.environ["GH59_FAKE_REPO"]
    action = os.environ.get("GH59_FAKE_POST_ACTION", "")
    if action == "dirty":
        open(os.path.join(repo, "dirty.txt"), "w").write("dirty")
    elif action == "drift":
        path = os.path.join(repo, "Cargo.toml")
        open(path, "a").write("\n# drift\n")
        subprocess.run(["git", "-C", repo, "add", "Cargo.toml"], check=True)
        subprocess.run(["git", "-C", repo, "commit", "-q", "-m", "drift"], check=True)
'''
FAKE_RUSTC = '''#!/usr/bin/env python3
print("rustc 1.95.0-nightly (fixture)\\nrelease: 1.95.0-nightly\\ncommit-date: 2026-02-11\\nhost: fixture-target\\nLLVM version: 22.1.0")
'''


class Fixture:
    def __init__(self, root: Path, changed_lines: int = 10) -> None:
        self.root = root
        self.repo = root / "repo"
        self.repo.mkdir(parents=True)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname="fixture"\nversion="0.1.0"\nedition="2021"\n', encoding="utf-8"
        )
        (self.repo / "Cargo.lock").write_text(
            'version = 3\n\n[[package]]\nname = "fixture"\nversion = "0.1.0"\n', encoding="utf-8"
        )
        for path in CRITICAL:
            source = self.repo / path
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("pub fn covered(value: bool) -> usize { if value { 1 } else { 2 } }\n")
        removal = self.repo / "src/removal_only.rs"
        removal.write_text("pub fn kept() {}\npub fn removed() {}\n", encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-q", "-m", "base")
        self.base = self.git("rev-parse", "HEAD").stdout.strip()
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        removal.write_text("pub fn kept() {}\n", encoding="utf-8")
        changed = self.repo / "src/changed.rs"
        changed.write_text(
            "".join(f"pub const LINE_{line}: usize = {line};\n" for line in range(1, changed_lines + 1)),
            encoding="utf-8",
        )
        module_only = self.repo / "src/renderer/mod.rs"
        module_only.parent.mkdir()
        module_only.write_text(
            "pub use error::{DynamicFrameError, TextCoordinateError, TextProjectionError, TextRenderError};\n", encoding="utf-8",
        )
        (self.repo / "src/tests.rs").write_text(
            "#[test]\nfn fixture_only() {}\n", encoding="utf-8",
        )
        self.git("add", ".")
        self.git("commit", "-q", "-m", "head")
        self.refresh_head()
        self.fake_bin = root / "fake-bin"
        self.fake_bin.mkdir()
        self.make_executable(self.fake_bin / "cargo", FAKE_CARGO)
        self.make_executable(self.fake_bin / "rustc", FAKE_RUSTC)
        self.make_executable(self.fake_bin / "rustup", '#!/bin/sh\n[ "$1" = which ] || exit 9\nprintf "%s/%s\\n" "$GH59_FAKE_BIN" "$2"\n')
        self.make_executable(self.fake_bin / "cargo-llvm-cov", '#!/bin/sh\nexec cargo +nightly "$@"\n')
        self.llvm_template = root / "llvm-template.json"
        self.xml_template = root / "xml-template.xml"
        self.metadata_template = root / "metadata-template.json"
        package_id = "path+file://fixture#0.1.0"
        self.metadata_template.write_text(json.dumps({
            "packages": [{"id": package_id, "name": "fixture", "manifest_path": str(self.repo / "Cargo.toml")}],
            "workspace_members": [package_id], "version": 1,
        }))
        self.counter = 0
        self.changed_lines = changed_lines
        self.write_templates()

    def make_executable(self, path: Path, content: str) -> None:
        path.write_text(content, encoding="utf-8")
        path.chmod(0o755)

    def git(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", "-C", str(self.repo), *args], text=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True,
        )

    def refresh_head(self) -> None:
        self.head = self.git("rev-parse", "HEAD").stdout.strip()
        self.merge_base = self.git("merge-base", self.head, self.base).stdout.strip()

    @staticmethod
    def summary(count: int, covered: int, notcovered: bool = False) -> dict:
        percent = 0.0 if count == 0 else covered / count * 100.0
        value = {"count": count, "covered": covered, "percent": percent}
        if notcovered:
            value["notcovered"] = count - covered
        return value

    def raw_record(
        self, path: str, line_count: int, covered: int, critical: bool = False,
    ) -> dict:
        segments = [item for line in range(1, line_count + 1) for item in (
            [line, 1, 1 if line <= covered else 0, True, True, False],
            [line, 10, 0, False, False, False],
        )]
        branches = [[1, 1, 1, 2, 1, 1, 0, 0, 4]] if critical else []
        return {
            "filename": str(self.repo / path),
            "segments": segments,
            "branches": branches,
            "expansions": [],
            "mcdc_records": [],
            "summary": {
                "lines": self.summary(line_count, covered),
                "functions": self.summary(1, 1), "instantiations": self.summary(1, 1),
                "regions": self.summary(line_count, covered, True),
                "branches": self.summary(2, 2, True) if critical else self.summary(0, 0, True),
                "mcdc": self.summary(0, 0, True),
            },
        }

    def write_templates(self, mode: str = "valid", changed_covered: int | None = None) -> None:
        if changed_covered is None:
            changed_covered = self.changed_lines
        records = [self.raw_record(path, 1, 1, True) for path in CRITICAL]
        records.append(self.raw_record("src/removal_only.rs", 1, 1))
        records.append(self.raw_record("src/changed.rs", self.changed_lines, changed_covered))
        functions = []
        for path, record in zip((*CRITICAL, "src/removal_only.rs", "src/changed.rs"), records):
            line_count = record["summary"]["lines"]["count"]
            covered = record["summary"]["lines"]["covered"]
            function_branches = [list(item) for item in record["branches"]]
            functions.append({
                "name": f"fixture::{path}", "count": 1, "filenames": [str(self.repo / path)],
                "regions": [[line, 1, line, 10, 1 if line <= covered else 0, 0, 0, 0] for line in range(1, line_count + 1)],
                "branches": function_branches, "mcdc_records": [],
            })
        raw = {
            "data": [{"files": records, "functions": functions, "totals": {}}],
            "type": "llvm.coverage.json.export",
            "version": "3.1.0",
            "cargo_llvm_cov": {"version": "0.8.7", "manifest_path": str(self.repo / "Cargo.toml")},
        }
        coverage = ET.Element("coverage", {"version": "2.0.3", "timestamp": "1"})
        sources = ET.SubElement(coverage, "sources")
        ET.SubElement(sources, "source").text = str(self.repo)
        package = ET.SubElement(ET.SubElement(coverage, "packages"), "package", {"name": "fixture"})
        classes = ET.SubElement(package, "classes")
        for path in CRITICAL:
            klass = ET.SubElement(classes, "class", {"filename": path})
            lines = ET.SubElement(klass, "lines")
            ET.SubElement(lines, "line", {
                "number": "1", "hits": "1", "branch": "true", "condition-coverage": "100% (2/2)",
            })
        klass = ET.SubElement(classes, "class", {"filename": "src/removal_only.rs"})
        lines = ET.SubElement(klass, "lines")
        ET.SubElement(lines, "line", {"number": "1", "hits": "1", "branch": "false"})
        klass = ET.SubElement(classes, "class", {"filename": "src/changed.rs"})
        lines = ET.SubElement(klass, "lines")
        for line in range(1, self.changed_lines + 1):
            ET.SubElement(lines, "line", {
                "number": str(line), "hits": "1" if line <= changed_covered else "0", "branch": "false",
            })
        if mode == "wrong_tool":
            raw["cargo_llvm_cov"]["version"] = "0.8.6"
        elif mode == "wrong_schema":
            raw["version"] = "2.0.1"
        elif mode == "summary_only":
            records[0].pop("segments")
            records[0].pop("branches")
        elif mode == "missing_changed":
            records.pop()
        elif mode in ("genuine_summary_domains", "inflated_function_support"):
            functions.append({
                "name": f"{functions[0]['name']}::monomorphized",
                "count": 1,
                "filenames": list(functions[0]["filenames"]),
                "regions": [list(region) for region in functions[0]["regions"]],
                "branches": [list(branch) for branch in functions[0]["branches"]],
                "mcdc_records": [],
            })
            if mode == "genuine_summary_domains":
                functions[-1]["regions"][0][1] = 2
                records[0]["segments"].append([1, 2, 1, True, True, False])
                for kind in ("lines", "functions", "instantiations", "regions"):
                    records[0]["summary"][kind] = self.summary(2, 2, kind == "regions")
                first_class = classes.find("class")
                first = first_class.find("./lines/line")
                assert first_class is not None and first is not None
                first.set("condition-coverage", "100% (4/4)")
                method_lines = ET.SubElement(ET.SubElement(ET.SubElement(first_class, "methods"), "method"), "lines")
                ET.SubElement(method_lines, "line", {"number": "999", "hits": "0"})
        elif mode == "excess_line_summary":
            records[0]["summary"]["lines"] = self.summary(211, 211)
        elif mode == "huge_summary_integer":
            records[0]["summary"]["lines"] = self.summary(10**400, 10**400)
        elif mode == "huge_summary_percent":
            records[0]["summary"]["lines"]["percent"] = 10**400
        elif mode == "region_beyond_source":
            functions[0]["regions"][0][2] = 2
        elif mode == "huge_segment_column":
            records[0]["segments"][0][1] = 10**400
        elif mode == "odd_branch_summary":
            records[0]["summary"]["branches"] = self.summary(1, 1, True)
        elif mode in ("xml_only_branch_line", "xml_only_positive_line", "xml_only_unbounded_branch_line"):
            number = self.changed_lines + 1 if mode == "xml_only_unbounded_branch_line" else self.changed_lines - 1; line_count = self.changed_lines - (mode != "xml_only_unbounded_branch_line")
            functions[-1]["regions"] = [region for region in functions[-1]["regions"] if region[0] != number]; records[-1]["segments"] = [segment for segment in records[-1]["segments"] if segment[0] != number]
            records[-1]["summary"]["lines"] = self.summary(line_count, line_count)
            records[-1]["summary"]["regions"] = self.summary(line_count, line_count, True)
            branch = [[number, 1, max(number, self.changed_lines), 2, 1, 0, 0, 0, 4], [number, 1, max(number, self.changed_lines), 2, 0, 1, 0, 0, 4]]
            records[-1]["branches"].extend(branch)
            functions[-1]["branches"].extend(branch)
            records[-1]["summary"]["branches"] = self.summary(2, 2, True)
            last = list(list(classes)[-1].find("./lines"))[-2]
            last.set("number", str(number))
            last.set("hits", "1" if mode == "xml_only_positive_line" else "0")
            last.set("branch", "true")
            last.set("condition-coverage", "100% (2/2)")
        elif mode == "branch_summary_mismatch":
            records[0]["summary"]["branches"] = self.summary(2, 1, True)
        elif mode == "branch_xml_mismatch":
            first = coverage.find("./packages/package/classes/class/lines/line")
            assert first is not None
            first.set("condition-coverage", "50% (1/2)")
        elif mode == "zero_branch":
            records[0]["branches"] = []
            functions[0]["branches"] = []
            records[0]["summary"]["branches"] = self.summary(0, 0, True)
            first = coverage.find("./packages/package/classes/class/lines/line")
            assert first is not None
            first.set("branch", "false")
            first.attrib.pop("condition-coverage")
        elif mode == "critical_uncovered":
            functions[0]["count"] = 0; functions[0]["branches"][0][4:6] = [0, 0]
            functions[0]["regions"][0][4] = 0; records[0]["branches"][0][4:6] = [0, 0]; records[0]["summary"]["branches"] = self.summary(2, 0, True)
            for kind in ("lines", "functions", "instantiations", "regions"):
                records[0]["summary"][kind] = self.summary(1, 0, kind == "regions")
            first = coverage.find("./packages/package/classes/class/lines/line")
            assert first is not None
            first.set("hits", "0"); first.set("condition-coverage", "0% (0/2)")
        elif mode == "zero_detail_line":
            records[0]["segments"][0][3] = False
        elif mode == "zero_function_branch":
            functions[0]["branches"][0][4] = 0
            functions[0]["branches"][0][5] = 0
        elif mode == "extra_zero_function_branch":
            functions[0]["branches"].append([1, 5, 1, 6, 0, 0, 0, 0, 4])
        elif mode == "extra_file_branch":
            records[0]["branches"].append([1, 5, 1, 6, 1, 1, 0, 0, 4])
        elif mode == "path_escape":
            records[0]["filename"] = str(self.root / "outside.rs")
        elif mode == "duplicate_source":
            records.append(dict(records[0]))
        elif mode == "missing_changed_xml":
            classes.remove(list(classes)[-1])
        elif mode == "missing_changed_line":
            list(classes)[-1].find("./lines").remove(list(list(classes)[-1].find("./lines"))[-1])
        elif mode == "missing_removal":
            records.pop(-2); functions.pop(-2); classes.remove(list(classes)[-2])
        elif mode == "wrong_xml_version":
            coverage.set("version", "1.0")
        elif mode == "branch_percent_mismatch":
            first = coverage.find("./packages/package/classes/class/lines/line")
            assert first is not None
            first.set("condition-coverage", "0% (2/2)")
        elif mode == "notcovered_mismatch":
            records[0]["summary"]["branches"]["notcovered"] = 1
        for kind in checker.METRIC_KINDS:
            count = sum(record["summary"][kind]["count"] for record in records)
            covered = sum(record["summary"][kind]["covered"] for record in records)
            raw["data"][0]["totals"][kind] = self.summary(
                count, covered, kind in ("regions", "branches", "mcdc"),
            )
        root_lines = root_covered = root_branches = root_branch_covered = 0
        for klass in classes.findall("class"):
            lines = klass.findall("./lines/line")
            line_counts = (len(lines), sum(int(line.get("hits", "0")) > 0 for line in lines))
            branch_counts = [0, 0]
            for line in lines:
                match = checker.CONDITION_RE.fullmatch(line.get("condition-coverage", ""))
                if line.get("branch") == "true" and match:
                    branch_counts[1] += int(match.group(2)); branch_counts[0] += int(match.group(3))
            klass.set("line-rate", str(line_counts[1] / line_counts[0] if line_counts[0] else 0.0))
            klass.set("branch-rate", str(branch_counts[1] / branch_counts[0] if branch_counts[0] else 0.0))
            root_lines += line_counts[0]; root_covered += line_counts[1]
            root_branches += branch_counts[0]; root_branch_covered += branch_counts[1]
        package.set("line-rate", str(root_covered / root_lines if root_lines else 0.0))
        package.set("branch-rate", str(root_branch_covered / root_branches if root_branches else 0.0))
        coverage.attrib.update({
            "lines-valid": str(root_lines), "lines-covered": str(root_covered),
            "branches-valid": str(root_branches), "branches-covered": str(root_branch_covered),
            "line-rate": package.get("line-rate", "0"), "branch-rate": package.get("branch-rate", "0"),
        })
        if mode == "empty_json":
            self.llvm_template.write_bytes(b"")
        elif mode == "malformed_json":
            self.llvm_template.write_bytes(b"{")
        elif mode == "duplicate_json_key":
            encoded = json.dumps(raw).replace('"type":', '"type":"duplicate","type":', 1)
            self.llvm_template.write_text(encoded, encoding="utf-8")
        else:
            self.llvm_template.write_text(json.dumps(raw), encoding="utf-8")
        if mode == "empty_xml":
            self.xml_template.write_bytes(b"")
        elif mode == "malformed_xml":
            self.xml_template.write_bytes(b"<coverage>")
        else:
            self.xml_template.write_bytes(ET.tostring(coverage, encoding="utf-8", xml_declaration=True))

    def environment(self, action: str = "", symlink: bool = False, wrong_binary: bool = False) -> dict[str, str]:
        environment = os.environ.copy()
        environment.update({
            "PATH": f"{self.fake_bin}{os.pathsep}{environment['PATH']}",
            "PYTHONDONTWRITEBYTECODE": "1",
            "GH59_FAKE_LLVM": str(self.llvm_template),
            "GH59_FAKE_XML": str(self.xml_template),
            "GH59_FAKE_METADATA": str(self.metadata_template),
            "GH59_FAKE_REPO": str(self.repo), "GH59_FAKE_BIN": str(self.fake_bin),
            "GH59_FAKE_POST_ACTION": action,
            "GH59_FAKE_SYMLINK": "1" if symlink else "0",
            "GH59_FAKE_TOOL_VERSION": "cargo-llvm-cov 0.8.6" if wrong_binary else "cargo-llvm-cov 0.8.7",
            "CARGO_LLVM_COV_TARGET_DIR": "/forbidden", "__CARGO_LLVM_COV_RUSTC_WRAPPER": "bad",
            "LLVM_COV_FLAGS": "--bad",
        })
        return environment

    def identity_args(self) -> list[str]:
        return [
            "--repo-root", str(self.repo), "--head", self.head, "--base", self.base,
            "--merge-base", self.merge_base, "--trusted-base-ref", "origin/main",
        ]

    def invoke(self, args: list[str], action: str = "", symlink: bool = False, wrong_binary: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args], text=True, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, check=False, env=self.environment(action, symlink, wrong_binary),
        )

    def collect(self, mode: str = "valid", **options: object) -> tuple[subprocess.CompletedProcess[str], Path]:
        self.write_templates(mode, options.pop("changed_covered", None))
        self.counter += 1
        output = self.root / f"collection-{self.counter}"
        result = self.invoke(["collect", *self.identity_args(), "--output-dir", str(output)], **options)
        return result, output

    def evidence_args(self, output: Path, evidence: Path | None = None) -> list[str]:
        return [
            *self.identity_args(), "--llvm-json", str(output / "llvm-cov.json"),
            "--cobertura", str(output / "cobertura.xml"),
            "--receipt", str(output / "collection-receipt.json"),
            "--evidence", str(evidence or output / "coverage-evidence.json"),
        ]


class CoverageCheckerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir="/private/tmp")
        self.fixture = Fixture(Path(self.temporary.name))

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def assert_fails(self, result: subprocess.CompletedProcess[str], message: str) -> None:
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(message, result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def valid_collection(self) -> Path:
        result, output = self.fixture.collect()
        self.assertEqual(result.returncode, 0, result.stderr)
        return output

    def test_collect_produce_validate_bind_reviewed_formats_and_commands(self) -> None:
        output = self.valid_collection()
        receipt = json.loads((output / "collection-receipt.json").read_text())
        commands = receipt["collection"]["commands"]
        self.assertEqual(receipt["toolchain"]["cargo_llvm_cov"]["version"], "cargo-llvm-cov 0.8.7")
        self.assertTrue(all(command[0] == receipt["toolchain"]["cargo_llvm_cov"]["executable"]["launcher"] for command in commands))
        self.assertEqual(len(receipt["toolchain"]["cargo"]["executable"]["sha256"]), 64)
        self.assertEqual(receipt["collection"]["workspace"]["packages"], ["fixture"])
        self.assertIn("--branch", commands[1])
        self.assertTrue({"--locked", "--no-cfg-coverage", "--no-cfg-coverage-nightly"}.issubset(commands[1]))
        self.assertEqual(commands[2][2], "report")
        self.assertEqual(commands[2][3:5], ["--package", "fixture"])
        produced = self.fixture.invoke(["produce", *self.fixture.evidence_args(output)])
        self.assertEqual(produced.returncode, 0, produced.stderr)
        validated = self.fixture.invoke(["validate", *self.fixture.evidence_args(output)])
        self.assertEqual(validated.returncode, 0, validated.stderr)
        evidence = json.loads((output / "coverage-evidence.json").read_text())
        self.assertEqual(evidence["decision"], "allowed")
        self.assertEqual(evidence["changed_executable"]["total"], 10)
        module = next(item for item in evidence["changed_executable"]["files"] if item["path"] == "src/renderer/mod.rs")
        self.assertEqual((module["added"], module["executable"], module["covered"]), (1, 0, 0))
        self.assertNotIn("src/tests.rs", {item["path"] for item in evidence["changed_executable"]["files"]})
        self.assertEqual(len(evidence["critical"]), 4)

    def test_diff_state_machine_treats_header_like_hunk_bodies_as_added_lines(self) -> None:
        diff = (
            b"diff --git a/src/odd.rs b/src/odd.rs\n--- /dev/null\n+++ b/src/odd.rs\n"
            b"@@ -0,0 +1,2 @@\n+++ b/not-a-header\n++ /dev/null\n"
        )
        self.assertEqual(checker.parse_changed_lines(diff, self.fixture.repo), {"src/odd.rs": {1, 2}})

    def test_carriage_return_cannot_inject_a_diff_header(self) -> None:
        diff = (
            b"diff --git a/data.txt b/data.txt\n--- a/data.txt\n+++ b/data.txt\n"
            b"@@ -0,0 +1 @@\n+value\r+++ b/src/reconciler/identity.rs\n"
            b"@@ -0,0 +99 @@\n+not-production\n"
        )
        with self.assertRaises(checker.CheckError):
            checker.parse_changed_lines(diff, self.fixture.repo)

    def test_diff_state_machine_handles_delete_and_rename_into_src(self) -> None:
        diff = (
            b"diff --git a/old.rs b/old.rs\n--- a/old.rs\n+++ /dev/null\n@@ -1 +0,0 @@\n-old\n"
            b"diff --git a/misc/x b/src/renamed.rs\n--- /dev/null\n+++ b/src/renamed.rs\n"
            b"@@ -0,0 +1 @@\n+new\n"
        )
        self.assertEqual(checker.parse_changed_lines(diff, self.fixture.repo), {"src/renamed.rs": {1}})
        _repo, snapshot, _raw = checker.repository_snapshot(
            str(self.fixture.repo), self.fixture.head, self.fixture.base,
            self.fixture.merge_base, "origin/main",
        )
        self.assertIn("--no-renames", snapshot["diff_command"])

    def test_ambient_inter_hunk_context_is_overridden(self) -> None:
        self.fixture.git("config", "diff.interHunkContext", "99")
        _repo, snapshot, _raw = checker.repository_snapshot(
            str(self.fixture.repo), self.fixture.head, self.fixture.base,
            self.fixture.merge_base, "origin/main",
        )
        self.assertIn("--inter-hunk-context=0", snapshot["diff_command"])

    def test_binary_attribute_cannot_hide_changed_rust_path(self) -> None:
        info = self.fixture.repo / ".git/info/attributes"
        info.write_text("*.rs binary\n", encoding="utf-8")
        _repo, snapshot, diff = checker.repository_snapshot(
            str(self.fixture.repo), self.fixture.head, self.fixture.base,
            self.fixture.merge_base, "origin/main",
        )
        self.assertIn("--text", snapshot["diff_command"])
        self.assertIn("src/changed.rs", checker.parse_changed_lines(diff, self.fixture.repo))

    def test_hidden_index_flags_and_grafts_fail_closed(self) -> None:
        for option in ("--assume-unchanged", "--skip-worktree"):
            self.tearDown(); self.setUp()
            self.fixture.git("update-index", option, "Cargo.toml")
            result, _output = self.fixture.collect()
            self.assert_fails(result, "hidden state flags")
        self.tearDown(); self.setUp()
        graft = self.fixture.repo / ".git/info/grafts"
        graft.write_text(f"{self.fixture.base}\n", encoding="utf-8")
        result, _output = self.fixture.collect()
        self.assert_fails(result, "grafts are forbidden")
        self.tearDown(); self.setUp()
        manifest = self.fixture.repo / "Cargo.toml"
        before = manifest.stat()
        manifest.write_text(manifest.read_text().replace("fixture", "fixturE"), encoding="utf-8")
        os.utime(manifest, ns=(before.st_atime_ns, before.st_mtime_ns))
        result, _output = self.fixture.collect()
        self.assert_fails(result, "bytes do not match the index")

    def test_tracked_mode_and_symlink_type_fail_closed(self) -> None:
        manifest = self.fixture.repo / "Cargo.toml"
        self.fixture.git("config", "core.fileMode", "false")
        manifest.chmod(0o755)
        self.assertEqual(self.fixture.git("status", "--porcelain").stdout, "")
        result, _output = self.fixture.collect()
        self.assert_fails(result, "mode does not match the index")

        self.tearDown(); self.setUp()
        target = self.fixture.repo / "tracked-target.txt"
        link = self.fixture.repo / "tracked-link"
        target.write_text("target\n", encoding="utf-8")
        link.symlink_to(target.name)
        self.fixture.git("add", "tracked-target.txt", "tracked-link")
        self.fixture.git("commit", "-q", "-m", "tracked symlink")
        self.fixture.refresh_head()
        checker.repository_snapshot(
            str(self.fixture.repo), self.fixture.head, self.fixture.base,
            self.fixture.merge_base, "origin/main",
        )
        link.unlink()
        link.write_text(target.name, encoding="utf-8")
        self.fixture.git("config", "core.symlinks", "false")
        self.assertEqual(self.fixture.git("status", "--porcelain").stdout, "")
        result, _output = self.fixture.collect()
        self.assert_fails(result, "type does not match the index")

        self.tearDown(); self.setUp()
        self.fixture.git(
            "update-index", "--add", "--cacheinfo",
            f"160000,{self.fixture.head},tracked-gitlink",
        )
        result, _output = self.fixture.collect()
        self.assert_fails(result, "unsupported tracked type")

    def test_git_replacement_objects_do_not_change_reviewed_diff(self) -> None:
        self.fixture.git("replace", self.fixture.base, self.fixture.head)
        _repo, snapshot, diff = checker.repository_snapshot(
            str(self.fixture.repo), self.fixture.head, self.fixture.base,
            self.fixture.merge_base, "origin/main",
        )
        self.assertEqual(snapshot["base_sha"], self.fixture.base)
        self.assertIn("src/changed.rs", checker.parse_changed_lines(diff, self.fixture.repo))

    def test_arbitrary_base_and_invalid_shas_fail(self) -> None:
        args = ["collect", *self.fixture.identity_args(), "--output-dir", str(self.fixture.root / "bad")]
        index = args.index("--base") + 1
        args[index] = self.fixture.head
        self.assert_fails(self.fixture.invoke(args), "base does not match trusted-base-ref")
        args = ["collect", *self.fixture.identity_args(), "--output-dir", str(self.fixture.root / "bad2")]
        args[args.index("--head") + 1] = "ABC"
        self.assert_fails(self.fixture.invoke(args), "lowercase SHA")
        args = ["collect", *self.fixture.identity_args(), "--output-dir", str(self.fixture.root / "bad3")]
        args[args.index("--trusted-base-ref") + 1] = "HEAD~1"
        self.assert_fails(self.fixture.invoke(args), "trusted-base-ref")

    def test_stale_collection_after_commit_fails(self) -> None:
        output = self.valid_collection()
        (self.fixture.repo / "src/later.rs").write_text("pub fn later() {}\n")
        self.fixture.git("add", ".")
        self.fixture.git("commit", "-q", "-m", "later")
        self.fixture.refresh_head()
        result = self.fixture.invoke(["produce", *self.fixture.evidence_args(output)])
        self.assert_fails(result, "receipt does not match")

    def test_post_collection_dirty_and_head_drift_fail_without_receipt(self) -> None:
        for action, message in (("dirty", "worktree is not clean"), ("drift", "HEAD does not match")):
            self.tearDown()
            self.setUp()
            result, output = self.fixture.collect(action=action)
            self.assert_fails(result, message)
            self.assertFalse((output / "collection-receipt.json").exists())

    def test_output_directory_preexistence_and_symlink_fail(self) -> None:
        existing = self.fixture.root / "existing"
        existing.mkdir()
        result = self.fixture.invoke(["collect", *self.fixture.identity_args(), "--output-dir", str(existing)])
        self.assert_fails(result, "must not preexist")
        link = self.fixture.root / "link"
        link.symlink_to(existing, target_is_directory=True)
        result = self.fixture.invoke(["collect", *self.fixture.identity_args(), "--output-dir", str(link)])
        self.assert_fails(result, "must not traverse a symlink")

    def test_wrong_binary_version_and_symlink_artifact_fail(self) -> None:
        result, _output = self.fixture.collect(wrong_binary=True)
        self.assert_fails(result, "exactly 0.8.7")
        result, output = self.fixture.collect(symlink=True)
        self.assert_fails(result, "must not traverse a symlink")
        self.assertFalse((output / "collection-receipt.json").exists())

    def test_wrong_schema_tool_provenance_and_summary_only_fail(self) -> None:
        for mode, message in (
            ("wrong_tool", "lacks cargo-llvm-cov 0.8.7"),
            ("wrong_schema", "type/version"),
            ("wrong_xml_version", "Cobertura XML type/version"),
            ("summary_only", "detailed segments/branches"),
        ):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_missing_changed_file_and_line_fail(self) -> None:
        result, _output = self.fixture.collect("missing_changed")
        self.assert_fails(result, "source record sets disagree")
        result, _output = self.fixture.collect("missing_changed_xml")
        self.assert_fails(result, "source record sets disagree")
        result, _output = self.fixture.collect("missing_changed_line")
        self.assert_fails(result, "detailed line coverage disagrees")
        result, _output = self.fixture.collect("missing_removal")
        self.assert_fails(result, "deletion-only production Rust path")

    def test_detailed_line_and_branch_mismatches_fail(self) -> None:
        for mode, message in (
            ("branch_xml_mismatch", "detailed branch coverage disagrees"),
            ("zero_branch", "zero detailed observations"),
            ("critical_uncovered", "not 100% covered"),
        ):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_genuine_json_and_cobertura_summary_domains_may_differ(self) -> None:
        conservative, _unused = self.fixture.collect("branch_summary_mismatch"); self.assertEqual(conservative.returncode, 0, conservative.stderr)
        result, output = self.fixture.collect("genuine_summary_domains")
        self.assertEqual(result.returncode, 0, result.stderr)
        produced = self.fixture.invoke(["produce", *self.fixture.evidence_args(output)])
        self.assertEqual(produced.returncode, 0, produced.stderr)
        critical = json.loads((output / "coverage-evidence.json").read_text())["critical"][0]
        self.assertEqual(critical["llvm"]["lines"], {"total": 1, "covered": 1})
        self.assertEqual(critical["cobertura"]["lines"], {"total": 1, "covered": 1})
        self.assertEqual(critical["llvm"]["branches"], {"total": 2, "covered": 2})
        self.assertEqual(critical["cobertura"]["branches"], {"total": 4, "covered": 4})

    def test_summary_counts_require_detailed_support(self) -> None:
        for mode, message in (
            ("excess_line_summary", "lines summary"),
            ("inflated_function_support", "instantiations summary"),
            ("huge_summary_integer", "64-bit integer"),
            ("huge_summary_percent", "summary is inconsistent"),
            ("region_beyond_source", "region exceeds source lines"),
            ("huge_segment_column", "segment shape or values"),
            ("odd_branch_summary", "branch summary count must be even"),
        ):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_added_executable_file_cannot_be_absent_from_both_artifacts(self) -> None:
        cases = (
            ("src/omitted_executable.rs", b"pub fn omitted() -> usize { 1 }\n"),
            ("src/renderer/mod.rs", b"use std::fmt; pub fn omitted() -> usize { 1 }\n"),
            ("src/renderer/mod.rs", b"use std::fmt;\rpub fn omitted() -> usize { 1 }\n"),
        )
        for index, (path, payload) in enumerate(cases):
            fixture = Fixture(self.fixture.root / f"omission-{index}")
            target = fixture.repo / path
            target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(payload)
            fixture.git("add", path); fixture.git("commit", "-q", "-m", "omitted executable"); fixture.refresh_head()
            result, _output = fixture.collect()
            self.assert_fails(result, "changed production Rust path is missing")

    def test_xml_only_uncovered_branch_line_counts_but_positive_and_unbounded_extras_fail(self) -> None:
        result, output = self.fixture.collect("xml_only_branch_line")
        self.assertEqual(result.returncode, 0, result.stderr)
        produced = self.fixture.invoke(["produce", *self.fixture.evidence_args(output)])
        self.assertEqual(produced.returncode, 0, produced.stderr)
        changed = json.loads((output / "coverage-evidence.json").read_text())["changed_executable"]
        self.assertEqual((changed["total"], changed["covered"]), (10, 9))
        for mode, message in (("xml_only_positive_line", "unsupported extra line observation"), ("xml_only_unbounded_branch_line", "branch shape or values")):
            result, _output = self.fixture.collect(mode); self.assert_fails(result, message)

    def test_real_export_semantics_and_summary_integrity(self) -> None:
        output = self.valid_collection()
        raw = json.loads((output / "llvm-cov.json").read_text())
        first, changed = raw["data"][0]["files"][0], raw["data"][0]["files"][-1]
        self.assertEqual((len(first["branches"]), first["summary"]["branches"]["count"]), (1, 2))
        self.assertEqual((len(changed["segments"]), changed["summary"]["lines"]["count"]), (20, 10))
        for mode, message in (("branch_percent_mismatch", "condition counts"), ("notcovered_mismatch", "notcovered")):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)
        for mode, message in (("zero_detail_line", "counted file segment support"), ("zero_function_branch", "branch details disagree"), ("extra_file_branch", "branch details disagree"), ("extra_zero_function_branch", "branch details disagree")):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_atomic_publish_is_no_clobber_and_cleans_postcheck_failure(self) -> None:
        target = self.fixture.root / "atomic.json"
        calls = 0
        def callback() -> None:
            nonlocal calls
            calls += 1
            if calls == 2:
                raise checker.CheckError("postcheck")
        with self.assertRaises(checker.CheckError):
            checker.atomic_write(target, b"{}\n", "test artifact", callback)
        self.assertFalse(target.exists())
        target.write_bytes(b"preserve")
        with self.assertRaises(checker.CheckError):
            checker.atomic_write(target, b"replace", "test artifact")
        self.assertEqual(target.read_bytes(), b"preserve")
        target.unlink(); calls = 0
        def tamper() -> None:
            nonlocal calls
            calls += 1
            if calls == 2: target.write_bytes(b"tampered\n")
        with self.assertRaises(checker.CheckError):
            checker.atomic_write(target, b"original\n", "test artifact", tamper)

    def test_duplicate_json_key_source_and_path_escape_fail(self) -> None:
        for mode, message in (
            ("duplicate_json_key", "duplicate object key"),
            ("duplicate_source", "duplicate normalized source"),
            ("path_escape", "path escapes repo-root"),
        ):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_malformed_and_empty_artifacts_fail(self) -> None:
        for mode, message in (
            ("empty_json", "LLVM JSON is empty"),
            ("malformed_json", "LLVM JSON is malformed"),
            ("empty_xml", "Cobertura XML is empty"),
            ("malformed_xml", "Cobertura XML is malformed"),
        ):
            result, _output = self.fixture.collect(mode)
            self.assert_fails(result, message)

    def test_exact_79_99_fails_and_80_percent_passes_integer_math(self) -> None:
        self.tearDown()
        self.temporary = tempfile.TemporaryDirectory(dir="/private/tmp")
        self.fixture = Fixture(Path(self.temporary.name), 10_000)
        result, _output = self.fixture.collect(changed_covered=7_999)
        self.assert_fails(result, "below 80%")
        self.tearDown()
        self.temporary = tempfile.TemporaryDirectory(dir="/private/tmp")
        self.fixture = Fixture(Path(self.temporary.name), 10)
        result, _output = self.fixture.collect(changed_covered=8)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_receipt_artifact_and_evidence_tampering_fail(self) -> None:
        output = self.valid_collection()
        receipt = output / "collection-receipt.json"
        receipt.write_bytes(b" " + receipt.read_bytes())
        self.assert_fails(
            self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]),
            "receipt does not match",
        )
        output = self.valid_collection()
        llvm = output / "llvm-cov.json"
        llvm.write_bytes(llvm.read_bytes() + b" ")
        self.assert_fails(
            self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]),
            "timestamp is outside the collection window",
        )
        output = self.valid_collection()
        self.assertEqual(self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]).returncode, 0)
        evidence = output / "coverage-evidence.json"
        evidence.write_bytes(b" " + evidence.read_bytes())
        self.assert_fails(
            self.fixture.invoke(["validate", *self.fixture.evidence_args(output)]),
            "evidence bytes do not match",
        )

    def test_missing_receipt_relative_paths_symlink_and_alias_fail(self) -> None:
        output = self.valid_collection()
        missing = output / "collection-receipt.json"
        missing.unlink()
        self.assert_fails(
            self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]), "does not exist",
        )
        output = self.valid_collection()
        args = self.fixture.evidence_args(output)
        args[args.index("--llvm-json") + 1] = "relative.json"
        self.assert_fails(self.fixture.invoke(["produce", *args]), "absolute path")
        output = self.valid_collection()
        evidence = output / "coverage-evidence.json"
        evidence.symlink_to(output / "llvm-cov.json")
        self.assert_fails(
            self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]), "symlink",
        )
        output = self.valid_collection()
        receipt = output / "collection-receipt.json"
        receipt.unlink()
        os.link(output / "llvm-cov.json", receipt)
        self.assert_fails(
            self.fixture.invoke(["produce", *self.fixture.evidence_args(output)]), "must not alias",
        )

    def test_dirty_worktree_fails_before_collection(self) -> None:
        (self.fixture.repo / "dirty.txt").write_text("dirty")
        result, _output = self.fixture.collect()
        self.assert_fails(result, "worktree is not clean")


if __name__ == "__main__":
    unittest.main()
