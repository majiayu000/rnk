# Changelog

All notable release notes for this repository live here. Older generated release
artifacts are also available from the GitHub Releases page.

## Unreleased

### Fixed

- Returned file content from the GLM chat example's approved `read_file` tool,
  capped at 100 lines and 16 KiB with an explicit truncation notice.

### Changed

- Prepared the release workflow for crates.io trusted publishing through GitHub
  OIDC, with fail-fast crates.io version checks before publish.

### Fixed

- GLM chat filename search now returns up to 20 relative regular-file paths as
  a JSON array, including matches under non-UTF-8 roots. Unix searches hold the
  root from authorization and use no-follow directories to prevent root,
  ancestor, or child symlink swaps from disclosing outside-root filenames.
  Unix reads also reuse the held root so ancestor replacements cannot disclose
  outside-root contents. Search fails closed with an unsupported-platform error
  on non-Unix systems where the approved root cannot be held safely.

## 0.19.3 - 2026-06-29

### Added

- Added a 5-minute getting started guide from an empty Cargo project to an
  interactive terminal UI.
- Added focused `rnk::prelude::widgets::*` and `rnk::prelude::testing::*`
  import surfaces for beginner component examples and tests.
- Added core component contract documentation and interaction/import-surface
  tests for Box, Text, TextInput, SelectInput, TextArea, and CommandPalette.
- Added CI and release gates for package contents and README docs/examples
  links so crates.io rendering stays usable.

### Changed

- Repositioned the README around `rnk` as a React-like Rust TUI framework for
  interactive CLI apps, agent UIs, dashboards, and chat-style terminal tools.
- Clarified when to choose `rnk`, Ratatui, Bubbletea, or Ink.
- Documented stable, advanced, and experimental API levels for the pre-1.0
  public surface.
- Consolidated duplicate PR quality workflows into the main CI workflow while
  preserving doc tests, coverage, and Miri coverage.

### Fixed

- Aligned MSRV documentation and contributor commands with the Rust 1.88 crate
  metadata and workspace-level CI checks.
- Excluded agent-facing files from the published root crate package.
- Converted README links to repository docs and examples to GitHub absolute URLs
  so they keep working from crates.io.

### Upgrade Notes

- Existing `rnk = "0.19.2"` users can upgrade to `rnk = "0.19.3"` for
  documentation, packaging, and additive prelude-surface improvements.
- No existing public modules were removed. The new prelude submodules are
  additive and intended to guide users toward a smaller app-facing surface.

### Compatibility Notes

- The top-level `rnk` crate continues to target Rust `1.88` and newer.
- Workspace helper crates keep their existing independent `0.1.0` package
  versions.

## 0.19.2 - 2026-05-31

### Changed

- Documented current limitations and caveats in the README.
- Linked the README to this changelog and the GitHub Releases page so release
  history is easier to find.

### Fixed

- Made `use_throttle` leading-edge state deterministic so trailing-edge values
  are not held back by worker-thread scheduling delays.

### Upgrade Notes

- No API, dependency, feature, or minimum-supported-Rust-version changes are
  included in this release.
- Existing `rnk = "0.19.1"` users can upgrade to `rnk = "0.19.2"` for the
  documentation updates and throttle scheduling fix.

### Compatibility Notes

- The top-level `rnk` crate targets Rust `1.88` and newer in the current
  checkout.
- Workspace helper crates keep their existing independent `0.1.0` package
  versions.

## Earlier Releases

See https://github.com/majiayu000/rnk/releases for release artifacts generated
before this changelog was added.
