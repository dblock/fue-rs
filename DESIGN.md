# Design

fue finds e-mail addresses of GitHub users and repository contributors from public GitHub data. It is a rewrite of the [fue](https://github.com/dblock/fue) Ruby gem that ships as a single binary, and it does not aim to be compatible with the gem's commands or output.

## Goals and Non-Goals

fue reads only public data through the GitHub REST and GraphQL APIs, including GitHub Enterprise Server. It does not scrape web pages or other sites, send e-mail, store tokens, or correlate linked accounts across services.

## Command Line

A single command takes one or more targets: `fue dblock` searches a user, and `fue dblock/fue` searches a repository's contributors. Targets may be written as `@login` or as `https://github.com/...` URLs. Results go to standard output; diagnostics and progress go to standard error.

## Output

fue shows every address it finds, grouped by person. Each address is one aligned line with the name used, the number of distinct commits, and the date it was last used. Real addresses come first, most recently used first, and `users.noreply` addresses, which do not receive e-mail, are marked `noreply`. A person without any address shows `(no e-mail found)` so they can be mentioned by login instead. `--verbose` adds every name used and each address's sources, and `--json` prints the same data for scripts.

```text
@octocat
  octocat@example.com                 The Octocat  412 commits, last 2026-11-23
  1+octocat@users.noreply.github.com  octocat      7 commits, last 2024-01-10     noreply
```

## Sources

For a user, fue merges four sources and records which ones found each address: the public profile e-mail, commit authors in recent public activity, commit history in repositories the user owns, has forked, or has contributed to, and `Signed-off-by:` lines in those commits. A commit seen by more than one source is counted once. Addresses are merged case-insensitively.

For a repository, fue reads the addresses contributors used in that repository's own history, including commit authors without a GitHub account, and lists people by commit count. `--limit` caps the number of people for large projects.

## Authentication

fue uses `--token`, then the `GH_TOKEN` or `GITHUB_TOKEN` environment variables, then `gh auth token` for the API's host, and otherwise fails with instructions to run `gh auth login`. `--api-url` selects a GitHub Enterprise Server API, and gh is asked for that host's token.

## Implementation

fue is a Rust 2024 crate requiring Rust 1.85 or newer; development is pinned to a current stable toolchain in `rust-toolchain.toml`. It uses `clap` for arguments, `reqwest` with rustls for HTTP so Linux binaries can be built statically without OpenSSL, `tokio` for concurrent requests, and `serde` for JSON. Tests run the binary against a local mock of the GitHub API selected with `--api-url`, so they need no network or credentials.

## Distribution

Releases will publish the crate to crates.io, attach prebuilt archives for macOS, Linux, and Windows to GitHub Releases, and update a formula in a shared `dblock/homebrew-tap` that installs those archives with `brew install dblock/tap/fue`. See [RELEASING](RELEASING.md).
