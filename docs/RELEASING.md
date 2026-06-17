# Release Policy

This repository is a Cargo workspace. Release checks must validate the workspace,
not only the root `rnk` crate.

## Publishable Packages

The release workflow treats these packages as publishable, in dependency order:

1. `rnk-style-core`
2. `rnk-style`
3. `rnk-icons`
4. `rnk`

`rnk-style` depends on `rnk-style-core` with both `path` and `version` so local
workspace development and crates.io publishing use the same package contract.

## Version Policy

The git tag version must match the root `rnk` package version. Helper crates keep
their own versions because they may change on a different cadence from the root
framework crate.

Before publishing, the release workflow reads package data from `cargo metadata`
and prints the publishable package versions. This avoids grepping only the root
manifest and missing helper crate metadata.

Because helper crates keep independent versions, not every root `rnk` release
has to bump every helper package. The publish step checks crates.io for each
publishable package version and skips packages that are already published.

## Release Gates

Every tag release must run the full workspace test gate:

```bash
cargo test --workspace --all-targets --all-features --locked --verbose
```

Each publishable package is dry-run validated immediately before it is published:

```bash
curl -fsS https://crates.io/api/v1/crates/<package>/<version>
cargo publish --dry-run --locked -p <package>
cargo publish --locked -p <package>
```

Already-published package versions are skipped. Publishing in dependency order
lets package-level dry-runs validate path-plus-version dependencies after their
upstream package has been released.
