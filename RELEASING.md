# Releasing

Release automation is not set up yet, and nothing here authorizes a release. The planned process is described below and will be replaced with verified instructions once it exists.

## Planned Process

Releases will use [rust-crate-release](https://github.com/dblock/rust-crate-release) from a manually dispatched GitHub Actions workflow on `master`. It will validate the crate, publish it to crates.io with Trusted Publishing, tag `vX.Y.Z`, create a GitHub Release, and prepare the next development version. Additional jobs will attach prebuilt archives and checksums for macOS, Linux, and Windows to the release and update the formula in `dblock/homebrew-tap`.

The first crates.io publication requires a one-time manual publish with a short-lived token, after which Trusted Publishing is configured for the `release` environment. Publication, tags, releases, credentials, and trusted-publisher setup each require explicit maintainer authorization. Never overwrite a published version or move a release tag.
