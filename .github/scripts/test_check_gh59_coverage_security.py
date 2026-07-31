#!/usr/bin/env python3
"""Adversarial regression tests for GH-59 coverage provenance."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from gh59_coverage_source import SourcePolicyError, analyze_rust_source, detect_coverage_control
from test_check_gh59_coverage import Fixture


class CoverageSecurityTests(unittest.TestCase):
    def test_joint_artifact_omission_cannot_hide_coverage_suppression(self) -> None:
        controls = (
            b"#[coverage(off)] fn hidden() {}\n",
            b"#[cfg(not(coverage))] fn hidden() {}\n",
            b"#[cfg_attr(coverage_nightly, coverage(off))] fn hidden() {}\n",
            b"#[automatically_derived] impl Hidden {}\n",
            b"#[unsafe(naked)] fn hidden() {}\n",
            b"fn hidden() { if cfg!(coverage) {} }\n",
        )
        self.assertTrue(all(analyze_rust_source(source).coverage_control for source in controls))
        benign = b'// #[coverage(off)]\nconst TEXT: &str = "#[cfg(coverage_nightly)]";\n'
        self.assertFalse(analyze_rust_source(benign).coverage_control)
        raw_literal_boundary = b'#![feature(coverage_attribute)]\nconst X: &[u8] = br"\\";\n#[coverage(off)]\nfn hidden() { panic!() }\nconst Q: char = \'"\';\n'
        self.assertTrue(analyze_rust_source(raw_literal_boundary).coverage_control)
        macro_meta = b'macro_rules! annotate { ($m:meta,$i:item) => { #[$m] $i } }\nannotate!(coverage(off), fn hidden() {});\n'
        self.assertTrue(analyze_rust_source(macro_meta).coverage_control)
        with tempfile.TemporaryDirectory(dir="/private/tmp") as temporary:
            fixture = Fixture(Path(temporary))
            changed = fixture.repo / "src/changed.rs"
            changed.write_text(
                changed.read_text(encoding="utf-8")
                + "#[coverage(off)]\npub fn hidden_from_both_exports() -> usize { 1 }\n",
                encoding="utf-8",
            )
            fixture.git("add", "src/changed.rs")
            fixture.git("commit", "-q", "-m", "attempt coverage suppression")
            fixture.refresh_head()

            result, _output = fixture.collect()

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("coverage suppression", result.stderr)
        with tempfile.TemporaryDirectory(dir="/private/tmp") as temporary:
            fixture = Fixture(Path(temporary))
            (fixture.repo / "src/lib.rs").write_text("#[cfg(not(coverage))]\nmod changed;\n", encoding="utf-8")
            fixture.git("add", "src/lib.rs"); fixture.git("commit", "-q", "-m", "unchanged parent control")
            fixture.base = fixture.git("rev-parse", "HEAD").stdout.strip(); fixture.git("update-ref", "refs/remotes/origin/main", fixture.base)
            changed = fixture.repo / "src/changed.rs"
            changed.write_text(changed.read_text(encoding="utf-8") + "pub const LINE_11: usize = 11;\n", encoding="utf-8")
            fixture.changed_lines = 11
            fixture.git("add", "src/changed.rs"); fixture.git("commit", "-q", "-m", "production-only child change"); fixture.refresh_head()
            result, _output = fixture.collect()
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("tracked HEAD Rust source contains explicit coverage suppression", result.stderr)

    def test_inline_test_padding_cannot_raise_production_percentage(self) -> None:
        policy = analyze_rust_source(
            b"fn ordinary() {}\n#[cfg(test)]\nfn padding() {}\n#[cfg(not(test))]\nfn production_only() {}\n"
        )
        self.assertEqual(policy.test_only_lines, frozenset((2, 3)))
        bare_test = analyze_rust_source(b"#[test]\nfn padding() {}\nfn production() {}\n")
        self.assertEqual(bare_test.test_only_lines, frozenset((1, 2)))
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"#[clippy::test]\nfn uncovered_production() { panic!() }\n")
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"#[cfg(all(any(test, unix), any(test, not(unix))))]\nfn padding() {}\n")
        self.assertFalse(detect_coverage_control(b"#[cfg(all(any(test, unix), any(test, not(unix))))]\nfn padding() {}\n"))
        self.assertFalse(detect_coverage_control(b"#[cfg(unix)] mod platform;\n#[cfg(test)] mod tests;\n"))
        generic = analyze_rust_source(b"fn ordinary() {\n#[cfg(test)]\nlet value: Result<\nVec<usize>,\nString,\n> = Ok(vec![1]);\nproduction();\n}\n")
        self.assertEqual(generic.test_only_lines, frozenset(range(2, 7)))
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"macro_rules! strip { (#[$m:meta] $i:item) => {$i} }\nstrip! { #[cfg(test)] fn production() {} }\n")
        raw_identifier = analyze_rust_source(b"fn pick() { match token {\n#[cfg(test)]\nr#fn if false => test_only(),\n_ => production(),\n} }\n")
        self.assertEqual(raw_identifier.test_only_lines, frozenset((2, 3)))
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"fn mixed() { #[cfg(test)] padding(); production(); }\n")
        with tempfile.TemporaryDirectory(dir="/private/tmp") as temporary:
            fixture = Fixture(Path(temporary))
            changed = fixture.repo / "src/changed.rs"
            changed.write_text(
                "#[cfg(test)]\n"
                "mod padding {\n"
                "fn padded() -> usize {\n"
                "let one = 1;\n"
                "let two = 2;\n"
                "let three = 3;\n"
                "one + two + three\n"
                "} // padded\n"
                "const TEST_ONLY: usize = 9;\n"
                "} // module\n"
                + "".join(f"pub const PROD_{line}: usize = {line};\n" for line in range(1, 11)),
                encoding="utf-8",
            )
            fixture.changed_lines = 20
            fixture.git("add", "src/changed.rs")
            fixture.git("commit", "-q", "-m", "pad coverage with inline tests")
            fixture.refresh_head()

            result, _output = fixture.collect(changed_covered=17)

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("below 80%", result.stderr)

    def test_collection_invokes_the_fingerprinted_plugin_directly(self) -> None:
        with tempfile.TemporaryDirectory(dir="/private/tmp") as temporary:
            fixture = Fixture(Path(temporary))
            fixture.make_executable(fixture.fake_bin / "cargo-llvm-cov", "#!/bin/sh\nexit 99\n")

            result, _output = fixture.collect()

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("cargo-llvm-cov command failed", result.stderr)
            cases = (("build.rs", "fn main() {}\n"), ("Cargo.toml", "# changed\n"), ("Cargo.lock", "# changed\n"), ("rust-toolchain.toml", '[toolchain]\nchannel = "nightly"\n'), (".cargo/config.toml", '[build]\nrustflags = ["--cfg", "coverage"]\n'))
            for index, (path, payload) in enumerate(cases):
                configured = Fixture(Path(temporary) / f"policy-{index}")
                target = configured.repo / path; target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text((target.read_text(encoding="utf-8") if target.exists() else "") + payload, encoding="utf-8")
                configured.git("add", path); configured.git("commit", "-q", "-m", "coverage-affecting config"); configured.refresh_head()
                rejected, _output = configured.collect()
                self.assertNotEqual(rejected.returncode, 0, rejected.stdout)
                self.assertIn("coverage-affecting build/tool configuration changed", rejected.stderr)


if __name__ == "__main__":
    unittest.main()
