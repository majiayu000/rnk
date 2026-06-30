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

The release workflow also validates the root `rnk` package file list before
publishing:

```bash
cargo package --list -p rnk --locked
```

The package list must not include agent-private files such as `AGENTS.md` or
`CLAUDE.md`, workflow files, or repository-only `docs/` and `specs/` content.
README links to repository docs and examples must use GitHub absolute URLs so
they keep working from crates.io.

Each publishable package is dry-run validated immediately before it is published:

```bash
curl -fsS https://crates.io/api/v1/crates/<package>/<version>
cargo publish --dry-run --locked -p <package>
cargo publish --locked -p <package>
```

Already-published package versions are skipped. Publishing in dependency order
lets package-level dry-runs validate path-plus-version dependencies after their
upstream package has been released.

When a helper crate is being published for the first time, a local dry-run of a
downstream package may fail until the upstream crate exists in the crates.io
index. For example, `cargo publish --dry-run -p rnk-style` requires
`rnk-style-core` to be visible on crates.io. The release workflow therefore
publishes in dependency order and waits for each newly published package version
to become visible before moving to the next package.

## crates.io Authentication

Releases use crates.io Trusted Publishing through GitHub OIDC. The workflow does
not read a long-lived `CARGO_REGISTRY_TOKEN` secret.

Before cutting the next release, a crates.io owner must configure Trusted
Publishing for every publishable crate:

| Crate | Repository | Workflow | Environment |
| --- | --- | --- | --- |
| `rnk-style-core` | `majiayu000/rnk` | `release.yml` | `release` |
| `rnk-style` | `majiayu000/rnk` | `release.yml` | `release` |
| `rnk-icons` | `majiayu000/rnk` | `release.yml` | `release` |
| `rnk` | `majiayu000/rnk` | `release.yml` | `release` |

The GitHub publish job has `contents: read` and `id-token: write` permissions so
`rust-lang/crates-io-auth-action@v1` can request a short-lived token. If the
trusted publisher is not configured, the auth step must fail before any
`cargo publish` command runs.

The publish plan checks crates.io before authentication. If all package versions
already exist, the workflow skips authentication and publishing so a rerun of an
already-published tag can still create or repair the GitHub Release job.

## Failure Recovery

- If package metadata, package contents, README links, tests, dry-run publishing,
  or crates.io lookup fail, fix the repository and cut a new tag when needed.
- If trusted publishing authentication fails, configure crates.io Trusted
  Publishing for the affected crate and rerun the tag workflow.
- If a crate publishes but the workflow stops before the next crate, rerun the
  workflow. Already-published versions are skipped and the remaining unpublished
  packages continue in dependency order.
