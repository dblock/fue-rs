# Releasing

Releases run from the manually dispatched [Release workflow](.github/workflows/release.yml) on `master`. It uses [rust-crate-release](https://github.com/dblock/rust-crate-release) v0.1.2 to validate the crate, publish it to crates.io with Trusted Publishing, push a release commit and `vX.Y.Z` tag, create a GitHub Release, and push the next development version. It then builds the binaries in [Binaries](.github/workflows/binaries.yml) from the tag and attaches their archives and SHA-256 checksums to the release. Merges never publish, and nothing in this document authorizes a release: publication, tags, credentials, and trusted-publisher setup each need explicit maintainer authorization. Never overwrite a published version or move a release tag.

## One-Time Setup

1. **First publication.** crates.io requires a new crate's first version to be published with an API token. Create a short-lived token with `publish-new` permission restricted to `fue`, then from a clean, up-to-date `master` run `cargo login`, `cargo publish --locked`, and `cargo logout`, pasting the token only at Cargo's prompt, and revoke it. Then open a PR that bumps `Cargo.toml` to the next patch version, refreshes `Cargo.lock` with `cargo check`, and dates the released CHANGELOG section. This first version gets no binaries; the next workflow release does.
2. **Trusted Publishing.** In this repository's **Settings > Environments**, create `release` and restrict it to `master`. On crates.io, open the crate's **Settings > Trusted Publishing** and add a GitHub publisher with owner `dblock`, repository `fue-rs`, workflow `release.yml`, and environment `release`.
3. **Branch rules.** The workflow pushes commits and tags directly to `master`. Remove the pull request requirement from the `master` ruleset, keeping its force-push and deletion protections. This also lets other writers push directly, so ordinary changes must still go through pull requests by convention.

## Release

Add the release notes to the `(Next)` section of [CHANGELOG](CHANGELOG.md), keeping `- Your contribution here.`, and merge them. Then dispatch the workflow on `master`:

```sh
gh workflow run release.yml --repo dblock/fue-rs --ref master
```

This releases the version in `Cargo.toml`; pass `-f version=X.Y.Z` to release another version. The Homebrew formula in [dblock/homebrew-tap](https://github.com/dblock/homebrew-tap) is updated from the new release by that repository's scheduled workflow, or immediately with `gh workflow run update.yml --repo dblock/homebrew-tap`.

## Verify and Recover

Check that crates.io lists the version, that the `vX.Y.Z` tag and GitHub Release exist with five archives and five checksums, that `master` has the next development version, and that `brew install dblock/tap/fue` installs it.

If the workflow fails, check what completed before retrying, following rust-crate-release's [recovery instructions](https://github.com/dblock/rust-crate-release/blob/v0.1.2/docs/recovery.md): a crate upload may already have succeeded. If only the Attach Binaries jobs failed, rerun the failed jobs from the same workflow run; they rebuild from the tag and replace any uploaded archives.
