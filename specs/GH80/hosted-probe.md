# GH80 hosted CI probe

This marker exists only to trigger a temporary pull request whose base is
`fix/GH80-stacked-pr-ci`.

The probe validates that `.github/workflows/ci.yml` dispatches the unchanged
CI job set for a non-`main` base. The probe must not be merged.
