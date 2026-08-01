#!/usr/bin/env python3
"""Adversarial regression tests for GH-59 coverage provenance."""

from __future__ import annotations

import tempfile
import time
import unittest
from pathlib import Path

import check_gh59_coverage as checker
from gh59_coverage_source import SourcePolicyError, analyze_rust_source, detect_coverage_control
from test_check_gh59_coverage import Fixture


class CoverageSecurityTests(unittest.TestCase):
    def test_component_commit_dates_do_not_define_toolchain_identity(self) -> None:
        cargo = {"release": "1.94.0-nightly", "commit-date": "2026-01-15", "host": "fixture-target"}
        rustc = {"release": "1.94.0-nightly", "commit-date": "2026-01-17", "host": "fixture-target"}
        self.assertTrue(checker.toolchain_provenance_matches(cargo, rustc))
        rustc["host"] = "other-target"
        self.assertFalse(checker.toolchain_provenance_matches(cargo, rustc))

    def test_fixture_roots_use_the_platform_temporary_directory(self) -> None:
        hard_coded_root = "/" + "private/tmp"
        for name in ("test_check_gh59_coverage.py", "test_check_gh59_coverage_security.py"):
            source = Path(__file__).with_name(name).read_text(encoding="utf-8")
            self.assertNotIn(hard_coded_root, source)

    def test_joint_artifact_omission_cannot_hide_coverage_suppression(self) -> None:
        controls = (
            b"#[coverage(off)] fn hidden() {}\n",
            b"#[cfg(not(coverage))] fn hidden() {}\n",
            b"#[cfg_attr(coverage_nightly, coverage(off))] fn hidden() {}\n",
            b"#[cfg_attr(all(), inline, coverage(off))] fn hidden() {}\n",
            b"#[cfg_attr(all(), inline, cfg_attr(all(), coverage(off)))] fn hidden() {}\n",
            b"#[automatically_derived] impl Hidden {}\n",
            b"#[unsafe(naked)] fn hidden() {}\n",
            b"fn hidden() { if cfg!(coverage) {} }\n",
        )
        self.assertTrue(all(analyze_rust_source(source).coverage_control for source in controls))
        benign = b'// #[coverage(off)]\nconst TEXT: &str = "#[cfg(coverage_nightly)]";\n'
        self.assertFalse(analyze_rust_source(benign).coverage_control)
        ordinary_identifiers = (
            b"fn coverage() {}\n",
            b"mod coverage;\n",
            b"fn threshold() { let coverage = 80; consume(coverage); }\n",
            b"fn coverage_nightly() -> bool { false }\n",
            b"fn invoke(off: bool) { coverage(off); }\n",
            b"fn negate(coverage: bool) { if !(coverage) {} }\n",
            b"fn wait(coverage_nightly: bool) { while !(coverage_nightly) {} }\n",
            b"fn result(naked: bool) -> bool { return !(naked); }\n",
            b"macro_rules! coverage { () => {} } coverage!();\n",
            b"#[lint_policy(coverage(80))] fn visible() {}\n",
            b'macro_rules! discard { ($x:literal) => {} } discard!("x"coverage);\n',
            b'macro_rules! discard { ($x:literal) => {} } discard!(r"x"coverage_nightly);\n',
            b"macro_rules! discard { ($x:literal) => {} } discard!(1naked);\n",
            b'#[path = "visible.rs"] mod visible;\n',
            b'#[path = "visible.rs"] mod tests;\n',
            b'fn visible(path: &str) { consume(path); }\n',
            b"fn include() {}\n",
            b"fn invoke(include: fn()) { include(); }\n",
            b"fn ordinary(include: usize) { let r#use = include; }\n",
            b"fn compare(a: bool, include: bool) { if a != (include) {} }\n",
            b"fn compare(macro_rules: bool, include: bool) { if macro_rules != (include) {} }\n",
            b"fn identity<'a>(include: &'a ()) -> impl Sized + use<'a> { include }\n",
            b"fn include() {}\nfn f<'a>() -> impl Fn() + use<'a> { || include() }\n",
            b'macro_rules! docs { ($doc:literal,$item:item) => { #[doc=$doc] $item } }\n',
        )
        self.assertTrue(all(not detect_coverage_control(source) for source in ordinary_identifiers))
        ambiguous_macro_tokens = (
            b"fn threshold() { let coverage = 80; assert_eq!(coverage, 80); }\n",
            b'fn visible(path: &str) { assert_eq!(path, "visible"); }\n',
            b"keep!(coverage(false));\n",
            b"keep!(all(coverage));\n",
        )
        self.assertTrue(all(detect_coverage_control(source) for source in ambiguous_macro_tokens))
        source_inclusion_controls = (
            b'include!("hidden.inc");\n',
            b'allow(unexpected_cfgs); use std::include as inc; inc!("hidden.inc");\n',
            b'pub use std::{include as inc}; inc!("hidden.inc");\n',
            b'macro_rules! call { ($m:ident) => { $m!("tests/hidden.rs"); } }\ncall!(include);\n',
            b'macro_rules! import { ($m:ident) => { use std::$m as inc; } }\nimport!(include); inc!("hidden.inc");\n',
            b'attach!(path = "hidden.inc", mod hidden;);\n',
            b'macro_rules! attach { ($m:ident,$p:literal,$i:item) => { #[$m=$p] $i } } attach!(path, "hidden.inc", mod hidden;);\n',
            b'macro_rules! attach { ($name:ident) => { mod $name; } } attach!(tests);\n',
            b'#[path = "hidden.inc"] mod hidden;\n',
            b'#[path = "../hidden.rs"] mod hidden;\n',
            b'#[path = "C:/outside/hidden.rs"] mod hidden;\n',
            b'#[path = "C:outside.rs"] mod hidden;\n',
            b'#[path = "tests/hidden.rs"] mod hidden;\n',
            b'mod tests { #[path = "hidden.rs"] mod hidden; }\n',
            b'mod tests;\n',
        )
        self.assertTrue(all(detect_coverage_control(source) for source in source_inclusion_controls))
        test_only_source_inclusion = (
            b'#[cfg(test)]\n#[path = "tests/hidden.rs"]\nmod hidden;\n',
            b'#[path = "tests/hidden.rs"]\n#[cfg(test)]\nmod hidden;\n',
            b'#[cfg(all(test, unix))]\n#[path = "tests/hidden.rs"]\nmod hidden;\n',
            b'#[cfg(test)]\nmod tests { #[path = "hidden.rs"] mod hidden; }\n',
            b'#[cfg(test)]\nmod tests;\n',
            b'macro_rules! attach { ($name:ident) => { mod $name; } }\n#[cfg(test)]\nattach!(tests);\n',
        )
        self.assertTrue(
            all(
                not detect_coverage_control(source)
                for source in test_only_source_inclusion
            )
        )
        raw_literal_boundary = b'#![feature(coverage_attribute)]\nconst X: &[u8] = br"\\";\n#[coverage(off)]\nfn hidden() { panic!() }\nconst Q: char = \'"\';\n'
        self.assertTrue(analyze_rust_source(raw_literal_boundary).coverage_control)
        macro_controls = (
            b'macro_rules! annotate { ($m:meta,$i:item) => { #[$m] $i } }\nannotate!(coverage(off), fn hidden() {});\n',
            b'macro_rules! annotate { ($i:item,$m:meta) => { #[$m] $i } }\nannotate!(fn hidden() {}, coverage(off));\n',
            b'macro_rules! annotate { ($i:item;$m:meta) => { #[$m] $i } }\nannotate!(fn hidden() {}; coverage(off));\n',
            b'macro_rules! annotate { ($m:meta=>$i:item) => { #[$m] $i } }\nannotate!(coverage(off) => fn hidden() {});\n',
            b'macro_rules! annotate { ($i:item,$m:meta) => { #[$m] $i } }\nannotate!(impl Hidden {}, automatically_derived);\n',
            b'macro_rules! annotate { ($m:meta=>$i:item) => { #[$m] $i } }\nannotate!(naked => unsafe extern "C" fn hidden() {});\n',
            b'macro_rules! annotate { ($i:item@$m:meta) => { #[$m] $i } }\nannotate!(pub fn hidden() {} @ coverage(off));\n',
            b'macro_rules! annotate { ($i:item $m:meta) => { #[$m] $i } }\nannotate!(pub fn hidden() {} coverage(off));\n',
            b'macro_rules! annotate { (($m:meta),$i:item) => { #[$m] $i } }\nannotate!((coverage(off)), fn hidden() {});\n',
            b'macro_rules! make { ($m:meta,$n:ident) => { #[$m] pub fn $n() {} } }\nmake!(coverage(off), hidden);\n',
            b'macro_rules! gate { ($p:meta,$i:item) => { #[cfg($p)] $i } }\ngate!(all(coverage), fn hidden() {});\n',
            b'macro_rules! gate { ($m:ident,$i:item)=>{#[cfg_attr(all(),$m(test))] $i} }\ngate!(cfg, fn padding() {});\n',
            b'macro_rules! gate { ($h:tt,$m:meta,$i:item)=>{$h[$m] $i} }\ngate!(#, cfg(test), fn padding() {});\n',
        )
        self.assertTrue(all(analyze_rust_source(source).coverage_control for source in macro_controls))
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
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
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
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
        inclusion_attempts = (
            (
                "macro-forwarded-path",
                {
                    "src/lib.rs": (
                        "macro_rules! attach { "
                        "($meta:ident, $path:literal, $item:item) => "
                        "{ #[$meta = $path] $item } }\n"
                        'attach!(path, "hidden.inc", mod hidden;);\n'
                    ),
                    "src/hidden.inc": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "production-excluded-path",
                {
                    "src/lib.rs": (
                        '#[path = "tests/hidden.rs"]\nmod hidden;\n'
                    ),
                    "src/tests/hidden.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "nested-production-excluded-path",
                {
                    "src/lib.rs": (
                        'mod tests { #[path = "hidden.rs"] mod hidden; }\n'
                    ),
                    "src/tests/hidden.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "default-production-excluded-module",
                {
                    "src/lib.rs": "mod tests;\n",
                    "src/tests.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "macro-forwarded-excluded-module",
                {
                    "src/lib.rs": (
                        "macro_rules! attach { "
                        "($name:ident) => { mod $name; } }\n"
                        "attach!(tests);\n"
                    ),
                    "src/tests.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "macro-forwarded-include",
                {
                    "src/lib.rs": (
                        'macro_rules! call { ($m:ident) => { $m!("tests/hidden.rs"); } }\n'
                        "call!(include);\n"
                    ),
                    "src/tests/hidden.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
            (
                "closure-parameter-path",
                {
                    "src/lib.rs": (
                        "fn f() { let _f = |\n#[cfg(test)]\n_x: usize\n| {\n"
                        '#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
                    ),
                    "src/tests/hidden.rs": "pub fn production() {}\n",
                },
            ),
            (
                "cfg-block-before-zero-parameter-closure",
                {
                    "src/lib.rs": (
                        "fn f() {\n#[cfg(test)]\n{ true; }\n|| {\n"
                        '#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
                    ),
                    "src/tests/hidden.rs": "pub fn production() {}\n",
                },
            ),
            (
                "cfg-parameter-in-closure-after-block",
                {
                    "src/lib.rs": (
                        "fn f() {\n{ () }\n|\n#[cfg(test)]\nx: usize\n| {\n"
                        '#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
                    ),
                    "src/tests/hidden.rs": "pub fn production() {}\n",
                },
            ),
            (
                "macro-wrapped-excluded-module",
                {
                    "src/lib.rs": (
                        "macro_rules! attach { "
                        "($item:item) => { mod tests { $item } } }\n"
                        "attach!(pub mod hidden;);\n"
                    ),
                    "src/tests/hidden.rs": "pub fn hidden() -> usize { 0 }\n",
                },
            ),
        )
        for name, sources in inclusion_attempts:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                fixture = Fixture(Path(temporary).resolve())
                for relative, source in sources.items():
                    target = fixture.repo / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_text(source, encoding="utf-8")
                fixture.git("add", ".")
                fixture.git("commit", "-q", "-m", name)
                fixture.refresh_head()
                result, _output = fixture.collect()
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("coverage suppression", result.stderr)
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
            hidden = fixture.repo / "src/tests/hidden.rs"
            hidden.parent.mkdir(parents=True, exist_ok=True)
            hidden.write_text("pub fn hidden() -> usize { 0 }\n", encoding="utf-8")
            (fixture.repo / "src/link").symlink_to("tests", target_is_directory=True)
            (fixture.repo / "src/lib.rs").write_text(
                '#[path = "link/hidden.rs"]\nmod hidden;\n',
                encoding="utf-8",
            )
            fixture.git("add", ".")
            fixture.git("commit", "-q", "-m", "tracked path directory symlink")
            fixture.refresh_head()
            result, _output = fixture.collect()
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("unsupported file type", result.stderr)
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
            (fixture.repo / ".gitignore").write_text(
                "src/ignored.rs\n", encoding="utf-8",
            )
            (fixture.repo / "src/lib.rs").write_text(
                '#[path = "ignored.rs"]\nmod ignored;\n',
                encoding="utf-8",
            )
            (fixture.repo / "src/ignored.rs").write_text(
                "pub fn ignored(value: bool) -> usize { if value { 1 } else { 0 } }\n",
                encoding="utf-8",
            )
            fixture.git("add", ".gitignore", "src/lib.rs")
            fixture.git("commit", "-q", "-m", "ignored production path target")
            fixture.refresh_head()
            result, _output = fixture.collect()
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("ignored Rust source", result.stderr)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            fixture = Fixture(root / "fixture")
            external = root / "external"
            external.mkdir()
            (external / "hidden.rs").write_text(
                "#[coverage(off)] pub fn hidden() -> usize { 0 }\n", encoding="utf-8",
            )
            (fixture.repo / ".gitignore").write_text("/src/link\n", encoding="utf-8")
            (fixture.repo / "src/lib.rs").write_text(
                '#[path = "link/hidden.rs"]\nmod hidden;\n', encoding="utf-8",
            )
            fixture.git("add", ".gitignore", "src/lib.rs")
            fixture.git("commit", "-q", "-m", "ignored production symlink")
            fixture.refresh_head()
            (fixture.repo / "src/link").symlink_to(external, target_is_directory=True)
            result, _output = fixture.collect()
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("ignored Rust source or symlink", result.stderr)

    def test_inline_test_padding_cannot_raise_production_percentage(self) -> None:
        policy = analyze_rust_source(
            b"fn ordinary() {}\n#[cfg(test)]\nfn padding() {}\n#[cfg(not(test))]\nfn production_only() {}\n"
        )
        self.assertEqual(policy.test_only_lines, frozenset((2, 3)))
        bare_test = analyze_rust_source(b"#[test]\nfn padding() {}\nfn production() {}\n")
        self.assertEqual(bare_test.test_only_lines, frozenset((1, 2)))
        for marker in ("\u200e", "\u200f"):
            spaced = analyze_rust_source(f"#{marker}[cfg(test)]\nfn padding() {{}}\n".encode())
            self.assertEqual(spaced.test_only_lines, frozenset((1, 2)))
        self.assertEqual(
            analyze_rust_source(b"#[cfg_attr(all(), cfg(test))]\nfn padding() {}\n").test_only_lines,
            frozenset((1, 2)),
        )
        self.assertFalse(
            analyze_rust_source(b"#[cfg_attr(test, cfg(test))]\nfn production() {}\n").test_only_lines
        )
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"#[cfg_attr(unix, cfg(test))]\nfn padding() {}\n")
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"#[clippy::test]\nfn uncovered_production() { panic!() }\n")
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"#[cfg(all(any(test, unix), any(test, not(unix))))]\nfn padding() {}\n")
        self.assertFalse(detect_coverage_control(b"#[cfg(all(any(test, unix), any(test, not(unix))))]\nfn padding() {}\n"))
        self.assertFalse(detect_coverage_control(b"#[cfg(unix)] mod platform;\n#[cfg(test)] mod tests;\n"))
        generic = analyze_rust_source(b"fn ordinary() {\n#[cfg(test)]\nlet value: Result<\nVec<usize>,\nString,\n> = Ok(vec![1]);\nproduction();\n}\n")
        self.assertEqual(generic.test_only_lines, frozenset(range(2, 7)))
        closure_parameter = analyze_rust_source(
            b"fn f() {\nlet _f = |\n#[cfg(test)]\n_x: usize\n| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden;\nhidden::production();\n};\n}\n'
        )
        self.assertEqual(closure_parameter.test_only_lines, frozenset((3, 4)))
        self.assertTrue(closure_parameter.coverage_control)
        closure_tail = analyze_rust_source(
            b"fn f() {\nlet _f = |first: usize,\n#[cfg(test)]\nlast: usize\n| { production(first); };\n}\n"
        )
        self.assertEqual(closure_tail.test_only_lines, frozenset((3, 4)))
        zero_parameter_closure = analyze_rust_source(
            b"fn f() {\n#[cfg(test)]\n{ true; }\n|| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
        )
        self.assertEqual(zero_parameter_closure.test_only_lines, frozenset((2, 3)))
        self.assertTrue(zero_parameter_closure.coverage_control)
        closure_after_block = analyze_rust_source(
            b"fn f() {\n{ () }\n|\n#[cfg(test)]\nx: usize\n| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
        )
        self.assertEqual(closure_after_block.test_only_lines, frozenset((4, 5)))
        self.assertTrue(closure_after_block.coverage_control)
        labelled_break = analyze_rust_source(
            b"fn f() { 'outer: loop { break 'outer |\n#[cfg(test)]\nx: usize\n| { production(); }; } }\n"
        )
        self.assertEqual(labelled_break.test_only_lines, frozenset((2, 3)))
        macro_before_closure = analyze_rust_source(
            b"fn f() {\nlet _ = stringify!(|);\nlet _f = |\n#[cfg(test)]\nx: usize\n| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n};\n}\n'
        )
        self.assertEqual(macro_before_closure.test_only_lines, frozenset((4, 5)))
        self.assertTrue(macro_before_closure.coverage_control)
        await_bitwise = analyze_rust_source(
            b"async fn f() { let _ = future.await | ({ let _f = |\n#[cfg(test)]\nx: usize\n| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; 0usize }) | mask; }\n'
        )
        self.assertEqual(await_bitwise.test_only_lines, frozenset((2, 3)))
        self.assertTrue(await_bitwise.coverage_control)
        for binary_prefix in (b"left | ", b"future.await | "):
            binary_or_closure = analyze_rust_source(
                b"async fn f() { let _ = " + binary_prefix + b"|\n#[cfg(test)]\nx: usize\n| {\n"
                b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}; }\n'
            )
            self.assertEqual(binary_or_closure.test_only_lines, frozenset((2, 3)))
            self.assertTrue(binary_or_closure.coverage_control)
        for let_keyword in (b"if", b"while"):
            let_pattern = analyze_rust_source(
                b"fn f() { " + let_keyword + b" let | closure = |\n#[cfg(test)]\nx: usize\n| {\n"
                b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n}\n{ production(); } }\n'
            )
            self.assertEqual(let_pattern.test_only_lines, frozenset((2, 3)))
            self.assertTrue(let_pattern.coverage_control)
        leading_or_then_closure = analyze_rust_source(
            b"fn f(e: E) { let _f = match e {\n| E::A => loop {},\nE::B => |\n"
            b"#[cfg(test)]\nx: usize\n| {\n"
            b'#[path = "tests/hidden.rs"] mod hidden; hidden::production();\n},\n}; }\n'
        )
        self.assertEqual(leading_or_then_closure.test_only_lines, frozenset((4, 5)))
        self.assertTrue(leading_or_then_closure.coverage_control)
        nested_closure = analyze_rust_source(
            b"fn f() { let _f = |outer: usize| |\n#[cfg(test)]\ninner: usize\n| { production(); }; }\n"
        )
        self.assertEqual(nested_closure.test_only_lines, frozenset((2, 3)))
        match_or_pattern = analyze_rust_source(
            b"fn f(e: E) { match e {\nE::A => (),\n#[cfg(test)] E::B | E::C => (),\n_ => (),\n} }\n"
        )
        self.assertEqual(match_or_pattern.test_only_lines, frozenset((3,)))
        enum_bitwise = analyze_rust_source(
            b"enum E {\nA,\n#[cfg(test)] B = 1 | 2,\nC,\n}\n"
        )
        self.assertEqual(enum_bitwise.test_only_lines, frozenset((3,)))
        comparison = analyze_rust_source(
            b"fn pick() { match 0 {\n#[cfg(test)]\n_ if c() => l() < r(),\n_ => production(),\n} }\n"
        )
        self.assertEqual(comparison.test_only_lines, frozenset((2, 3)))
        identifier_comparison = analyze_rust_source(
            b"fn pick() { match 0 {\n#[cfg(test)]\n_ if left < right => test_only(),\n"
            b"_ if greater > smaller => production(),\n_ => kept(),\n} }\n"
        )
        self.assertEqual(identifier_comparison.test_only_lines, frozenset((2, 3)))
        const_generic = analyze_rust_source(
            b"pub fn f<\n#[cfg(test)]\nconst N: usize,\nconst M: usize,\n>() -> usize {\nM\n}\n"
        )
        self.assertEqual(const_generic.test_only_lines, frozenset((2, 3)))
        last_generic = analyze_rust_source(
            b"pub fn f<U,\n#[cfg(test)]\nconst N: usize\n>() -> usize {\nproduction()\n}\n"
        )
        self.assertEqual(last_generic.test_only_lines, frozenset((2, 3)))
        impl_generic = analyze_rust_source(
            b"trait Tr { fn value() -> usize; }\nstruct S;\nimpl<'a,\n"
            b"#[cfg(test)]\n'b\n> Tr for S {\nfn value() -> usize { production() }\n}\n"
        )
        self.assertEqual(impl_generic.test_only_lines, frozenset((4, 5)))
        generic_default_comparison = analyze_rust_source(
            b"pub fn f<\n#[cfg(test)]\nconst N: bool = left < right,\n"
            b"const M: bool = greater > (smaller),\n>() -> bool { kept() }\n"
        )
        self.assertEqual(generic_default_comparison.test_only_lines, frozenset((2, 3)))
        struct_comparison = analyze_rust_source(
            b"fn f() { let _ = S {\n#[cfg(test)]\na: left < right,\n"
            b"b: greater > (smaller),\nc: kept,\n}; }\n"
        )
        self.assertEqual(struct_comparison.test_only_lines, frozenset((2, 3)))
        block_statement = analyze_rust_source(
            b"fn f() {\n#[cfg(test)]\nif true { test_only(); }\n-production();\nkept();\n}\n"
        )
        self.assertEqual(block_statement.test_only_lines, frozenset((2, 3)))
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(
                b"fn f() {\n#[cfg(test)]\nif true { test_only(); } -production();\n}\n"
            )
        absolute_path = analyze_rust_source(
            b"fn f() {\n#[cfg(test)]\nif true { test_only(); }\n"
            b"::std::mem::drop(production());\nkept();\n}\n"
        )
        self.assertEqual(absolute_path.test_only_lines, frozenset((2, 3)))
        self.assertFalse(analyze_rust_source(
            b"#[cfg_attr(test, cfg_attr(unix, cfg(test)))]\nfn production() {}\n"
        ).test_only_lines)
        stacked_test = analyze_rust_source(
            b"#[cfg(test)]\n#[tokio::test]\nasync fn check() {}\n"
        )
        self.assertEqual(stacked_test.test_only_lines, frozenset((1, 2, 3)))
        nested_cfg_attr = (
            b"#[" + b"cfg_attr(all()," * 1_500 + b"cfg(test)"
            + b")" * 1_500 + b"]\nfn padding() {}\n"
        )
        self.assertEqual(
            analyze_rust_source(nested_cfg_attr).test_only_lines,
            frozenset((1, 2)),
        )
        nested_condition = (
            b"#[cfg(" + b"not(" * 1_500 + b"test"
            + b")" * 1_500 + b")]\nfn padding() {}\n"
        )
        self.assertEqual(
            analyze_rust_source(nested_condition).test_only_lines,
            frozenset((1, 2)),
        )
        nested_test = analyze_rust_source(
            b"#[cfg(test)]\nmod tests { #[tokio::test] async fn check() {} }\n"
        )
        self.assertEqual(nested_test.test_only_lines, frozenset((1, 2)))
        self.assertEqual(
            analyze_rust_source("\ufeff#[cfg(test)]\nfn padding() {}\n".encode()).test_only_lines,
            frozenset((1, 2)),
        )
        self.assertEqual(
            analyze_rust_source(b"#!/usr/bin/env (\n#[cfg(test)]\nfn padding() {}\n").test_only_lines,
            frozenset((2, 3)),
        )
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"macro_rules! strip { (#[$m:meta] $i:item) => {$i} }\nstrip! { #[cfg(test)] fn production() {} }\n")
        self.assertFalse(
            analyze_rust_source(b"macro_rules! test_case { ($name:ident) => { #[test] fn $name() {} } }\n").coverage_control
        )
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"wrap! { #[test] fn padding() {} }\n")
        raw_identifier = analyze_rust_source(b"fn pick() { match token {\n#[cfg(test)]\nr#fn if false => test_only(),\n_ => production(),\n} }\n")
        self.assertEqual(raw_identifier.test_only_lines, frozenset((2, 3)))
        with self.assertRaises(SourcePolicyError):
            analyze_rust_source(b"fn mixed() { #[cfg(test)] padding(); production(); }\n")
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
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
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve(), changed_lines=20)
            changed = fixture.repo / "src/changed.rs"
            changed.write_text(
                "#[cfg_attr(unix, cfg(test))]\nmod padding {\n"
                + "".join(f"pub const PAD_{line}: usize = {line};\n" for line in range(1, 8))
                + "} // padded\n"
                + "".join(f"pub const PROD_{line}: usize = {line};\n" for line in range(1, 11)),
                encoding="utf-8",
            )
            fixture.git("add", "src/changed.rs")
            fixture.git("commit", "--amend", "-q", "--no-edit")
            fixture.refresh_head()
            result, _output = fixture.collect(changed_covered=17)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("cannot be classified", result.stderr)

    def test_adversarial_source_shapes_are_processed_in_linear_time(self) -> None:
        cases = (
            ("stacked attributes", b"#[cfg(test)]\n" * 8_000 + b"fn kept() {}\n", analyze_rust_source),
            ("nested macros", b"m!(" * 16_000 + b"x" + b")" * 16_000 + b";\n", detect_coverage_control),
            ("macro use tokens", b"m!(" + b"use " * 16_000 + b");\n", detect_coverage_control),
            ("nested closure ranges", b"|" + b"(|" * 24_000 + b"#[cfg(test)]x" + b"|)" * 24_000 + b"| 0;\n", detect_coverage_control),
            ("attribute pipe tokens", b"#[cfg_attr(any(),bogus(" + b"|(" * 32_000 + b"x" + b")|" * 32_000 + b"))]\nfn f() {}\n", analyze_rust_source),
            ("inner attribute pipe tokens", b"#![cfg_attr(any(),bogus(" + b"|(" * 16_000 + b"x" + b")|" * 16_000 + b"))]\nfn f() {}\n", analyze_rust_source),
        )
        for name, source, operation in cases:
            with self.subTest(name=name):
                started = time.perf_counter()
                operation(source)
                self.assertLess(time.perf_counter() - started, 5.0)

    def test_collection_invokes_the_fingerprinted_plugin_directly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Fixture(Path(temporary).resolve())
            fixture.make_executable(fixture.fake_bin / "cargo-llvm-cov", "#!/bin/sh\nexit 99\n")

            result, _output = fixture.collect()

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("cargo-llvm-cov command failed", result.stderr)
            cases = (("build.rs", "fn main() {}\n"), ("Cargo.toml", "# changed\n"), ("Cargo.lock", "# changed\n"), ("rust-toolchain.toml", '[toolchain]\nchannel = "nightly"\n'), (".cargo/config.toml", '[build]\nrustflags = ["--cfg", "coverage"]\n'))
            for index, (path, payload) in enumerate(cases):
                configured = Fixture(Path(temporary).resolve() / f"policy-{index}")
                target = configured.repo / path; target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text((target.read_text(encoding="utf-8") if target.exists() else "") + payload, encoding="utf-8")
                configured.git("add", path); configured.git("commit", "-q", "-m", "coverage-affecting config"); configured.refresh_head()
                rejected, _output = configured.collect()
                self.assertNotEqual(rejected.returncode, 0, rejected.stdout)
                self.assertIn("coverage-affecting build/tool configuration changed", rejected.stderr)


if __name__ == "__main__":
    unittest.main()
