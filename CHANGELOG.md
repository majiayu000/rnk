# Changelog

All notable release notes for this repository live here. Older generated release
artifacts are also available from the GitHub Releases page.

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

- The top-level `rnk` crate still targets Rust `1.85` and newer.
- Workspace helper crates keep their existing independent `0.1.0` package
  versions.

## Earlier Releases

See https://github.com/majiayu000/rnk/releases for release artifacts generated
before this changelog was added.
