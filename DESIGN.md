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

For a user, fue merges four sources and records which ones found each address. A commit seen by more than one source is counted once, and addresses are merged case-insensitively.

- **Profile:** the public e-mail on the user's profile, from `GET /users/{login}`, which also gives the user ID used to filter commits by author.
- **Activity:** the user's commits leading up to their 10 most recent public pushes, up to 10 commits each. Push events no longer list commits, so each push's head commit is looked up, with the lookups running concurrently.
- **History:** the user's commits on the default branch of repositories they own, including forks, and of repositories they contributed commits to, most recently pushed first. `--breadth` (default 10) limits repositories and `--depth` (default 1) limits commits per repository; both page past GitHub's limit of 100 per request. GitHub times out when filtering many commits by author at once, so each request asks for at most 200 commits, and a GraphQL request that fails with a 502, 503, or 504 is retried once. Repository pages are chained by cursor and fetched one after another, while each repository's remaining history is independent, so fue fetches up to eight histories at once per repository list. It picks the number of repositories per request, and so the commits per repository in that request, that needs the fewest round trips assuming every repository has `--depth` commits by the user: shallow searches list 100 repositories per request, and deep ones list them all with a few commits each, then fetch the rest of each history concurrently.
- **Signed-off-by:** addresses in `Signed-off-by:` lines of those commits that belong to the same person, which often reveal a real address behind a `noreply` commit author. Reviewers and others also sign off on commits, so by default (`--sign-offs own`) a sign-off counts only when it has the commit author's address, a name whose words all appear in the author's name or the other way around, ignoring case and punctuation, or an address the person used elsewhere, on another commit or their profile. Other sign-offs are listed separately, under `Signed off by others:` with `--verbose` and as `signed_off_by_others` in JSON, and never counted as the person's. `--sign-offs all` counts every sign-off as the person's, and `--sign-offs none` ignores them. Repository search applies the same rule to each contributor.

The profile is read first; activity and both history searches then run concurrently. A repository deleted since a push is skipped.

For a repository, fue lists contributors from `GET /repos/{owner}/{repo}/contributors?anon=1`, which orders them by commits and includes commit authors without a GitHub account by e-mail, and keeps the first `--limit` (default 10). For each contributor it reads up to `--depth` (default 100) of their commits in the repository's default branch, filtered by account or, without one, by e-mail, so each address is one they used in that repository. Up to eight contributors are searched at once, and they are shown in the order of the contributors list. An empty repository has no contributors.

## Authentication

fue uses `--token`, then the `GH_TOKEN` or `GITHUB_TOKEN` environment variables, then `gh auth token` for the API's host, and otherwise fails with instructions to run `gh auth login`. `--api-url` selects a GitHub Enterprise Server API, and gh is asked for that host's token.

## Implementation

fue is a Rust 2024 crate requiring Rust 1.85 or newer; development is pinned to a current stable toolchain in `rust-toolchain.toml`. It uses `clap` for arguments, `reqwest` with rustls for HTTP so Linux binaries can be built statically without OpenSSL, `tokio` for concurrent requests, and `serde` for JSON. GraphQL queries are `.graphql` files in `graphql/`, compiled into typed Rust with [graphql_client](https://crates.io/crates/graphql_client) and checked against GitHub's published schema, which is vendored unmodified with its provenance and CC BY 4.0 attribution in [graphql/README.md](graphql/README.md). A misspelled field or argument fails the build instead of a request. Requests go through fue's own client, which reads GitHub's `NOT_FOUND` error type to skip deleted repositories. Tests run the binary against a local [wiremock](https://crates.io/crates/wiremock) mock of the GitHub API selected with `--api-url`, so they need no network or credentials. Test dependencies need a newer Rust than the minimum, so CI builds fue on Rust 1.85 and runs tests on the pinned toolchain.

## Logo

The logo is a unicorn whose eye is an `@`, a nod to the original name, Finding Unicorn Engineers. It is original artwork, generated as `assets/logo.svg` by `examples/logo.rs` with no dependencies, and the `@` is drawn as strokes so it does not depend on fonts. To change it, edit the example and run `cargo run --example logo`; `cargo test` fails while the committed SVG differs from the generated one.

## Distribution

Releases publish the crate to crates.io with [rust-crate-release](https://github.com/dblock/rust-crate-release) and attach prebuilt archives with SHA-256 checksums to the GitHub Release: `tar.gz` for macOS on Apple silicon and Intel and for statically linked Linux on arm64 and x86_64, and `zip` for Windows on x86_64. Every pull request builds and runs the same binaries. A formula in the shared [dblock/homebrew-tap](https://github.com/dblock/homebrew-tap) installs those archives with `brew install dblock/tap/fue`. See [RELEASING](RELEASING.md).
