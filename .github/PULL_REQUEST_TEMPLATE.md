## Description

Brief description of changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Checklist

- [ ] Code follows project style (`cargo fmt --all -- --check`)
- [ ] Linter passes (`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A clippy::collapsible_if -A clippy::manual_is_multiple_of`)
- [ ] Tests pass (`cargo test --workspace --all-targets --all-features --locked`)
- [ ] Package contents checked when package surface changes (`cargo package --list -p rnk --locked`)
- [ ] README repository docs/examples links are GitHub absolute URLs, not package-relative `docs/` or `examples/` links
- [ ] Release-touching changes update `docs/RELEASING.md` and preserve trusted publishing / fail-fast gates
- [ ] New tests added for new functionality
- [ ] Documentation updated if needed

## Related Issues

Closes #

## Screenshots/Examples

If applicable, add screenshots or code examples.
