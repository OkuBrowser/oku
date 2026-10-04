# Releasing mainline

## Prerequisites

- Push access to `main` branch
- `CARGO_REGISTRY_TOKEN` secret configured in GitHub repo settings
  (Settings → Secrets and variables → Actions → New repository secret)

## Release Steps

### 1. Bump the version and create version PR

Make sure you are working on the most recent main branch.

Update the version in `Cargo.toml`:

```toml
[package]
version = "7.0.0"  # bump appropriately
```

Update `CHANGELOG.md` with the new version and changes.

Create the version PR:

```bash
git checkout -b chore/v7.0.0
git add Cargo.toml CHANGELOG.md
git commit -m "chore: v7.0.0"
git push origin chore/v7.0.0
```

PR title: `chore: v7.0.0`
PR description: Changelogs

### 2. Tag and push

After the PR has been merged, tag the commit.

```bash
git checkout main
git pull origin main
git tag v7.0.0
git push origin v7.0.0
```

This triggers the release workflow. That's it — CI handles the rest.

## What CI Does Automatically

When a `v*` tag is pushed, the [release workflow](.github/workflows/release.yml) runs:

1. **Creates a GitHub Release** with auto-generated release notes
2. **Publishes to crates.io** via `cargo publish`

## Post-Release Verification

- [ ] [GitHub Releases](https://github.com/pubky/mainline/releases) — new release with auto-generated notes
- [ ] [crates.io/crates/mainline](https://crates.io/crates/mainline) — new version visible

## Versioning

Follow [Semantic Versioning](https://semver.org/):

- **Patch** (6.1.x): bug fixes, no API changes
- **Minor** (6.x.0): new features, backwards-compatible
- **Major** (x.0.0): breaking API changes

## Troubleshooting

**CI build fails?** Fix the issue, then delete and re-push the tag:

```bash
git tag -d v7.0.0
git push origin :refs/tags/v7.0.0
# fix the issue, commit, push
git tag v7.0.0
git push origin v7.0.0
```

**cargo publish fails?** Check that `CARGO_REGISTRY_TOKEN` is set and not expired. Generate a new token at [crates.io/settings/tokens](https://crates.io/settings/tokens).
