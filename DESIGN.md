# Design

fue finds the e-mail addresses of GitHub users and repository contributors from public GitHub data. It is a rewrite of the [fue](https://github.com/dblock/fue) Ruby gem that ships as a single binary, and it is not compatible with the gem's commands or output.

## Goals and Non-Goals

fue reads only public data through the GitHub REST and GraphQL APIs, including on GitHub Enterprise Server. It does not scrape web pages or other sites, send e-mail, store tokens, or correlate linked accounts across services.

## Command Line

`fue` takes one or more targets: a user, such as `dblock`, or a repository, such as `dblock/fue`. Targets may also be written as `@login` or as `https://github.com/...` URLs. Results go to standard output, and diagnostics to standard error. While searching, a one-character spinner turns on standard error when it is a terminal, and is erased before anything else is printed. It is a small thread in `src/spinner.rs` rather than a progress-bar library, which would add several dependencies for one character.

## Output

Addresses are grouped by person, one aligned line each with the name used, the number of distinct commits, and the date last used. Real addresses come first, then most recently used. `noreply` addresses, which don't receive e-mail, are marked. A person without an address shows `(no e-mail found)`. `--verbose` adds every name and the sources of each address, and `--json` prints the same data for scripts.

```text
@octocat
  octocat@example.com                 The Octocat  412 commits, last 2026-11-23
  1+octocat@users.noreply.github.com  octocat      7 commits, last 2024-01-10     noreply
```

## User Search

fue merges four sources and records which ones found each address. Addresses are merged case-insensitively, and a commit seen by more than one source is counted once.

- **Profile:** the public e-mail on the profile (`GET /users/{login}`), which also gives the ID used to filter commits by author.
- **Activity:** the user's commits behind their 10 most recent public pushes, up to 10 each. Push events no longer list commits, so each push's head commit is looked up.
- **History:** the user's commits on the default branch of repositories they own, including forks, and repositories they contributed to, most recently pushed first, limited by `--breadth` and `--depth`.
- **Signed-off-by:** addresses in `Signed-off-by:` lines of those commits, which often reveal a real address behind a `noreply` author. See [Sign-Offs](#sign-offs).

## Repository Search

fue lists contributors from `GET /repos/{owner}/{repo}/contributors?anon=1`, ordered by commits and including commit authors without a GitHub account, and keeps the first `--limit`. For each one, it reads up to `--depth` of their commits on the repository's default branch, filtered by account or, without one, by e-mail, so each address is one they used in that repository. Sign-offs follow the same rules as for users.

## Sign-Offs

Reviewers and others sign off on commits too. By default (`--sign-offs own`), a sign-off is the person's only when it has:

- the commit author's address;
- a name whose words all appear in the author's name, or the other way around, ignoring case and punctuation; or
- an address the person used elsewhere in the search, on another commit or their profile.

Other sign-offs are never counted as the person's. They are listed under `Signed off by others:` with `--verbose` and as `signed_off_by_others` in JSON. `--sign-offs all` counts every sign-off, and `--sign-offs none` ignores them.

## Requests

- GitHub times out when filtering many commits by author in one request, so each request asks for at most 200 commits.
- Repository pages are chained by cursor, so they are fetched one after another, while each repository's remaining history is fetched concurrently, up to eight at a time. fue picks the repositories per request that needs the fewest round trips: shallow searches list 100 repositories per request, and deep searches list them with a few commits each, then fetch the rest concurrently.
- The profile is read first, then activity and both history searches run concurrently. Up to eight contributors of a repository are searched at once, and are shown in the contributors' order.
- A GraphQL request that fails with a 502, 503, or 504 is retried once. A repository deleted since a push is skipped.

## Authentication

fue uses `--token`, then `GH_TOKEN` or `GITHUB_TOKEN`, then `gh auth token` for the API's host, and otherwise fails with instructions to run `gh auth login`. `--api-url` selects a GitHub Enterprise Server API, whose token is read from gh for that host.

## Implementation

- A Rust 2024 crate that builds with Rust 1.85 and newer, developed on the toolchain pinned in `rust-toolchain.toml`. Test dependencies need a newer Rust, so CI builds on 1.85 and tests on the pinned toolchain.
- `clap` for arguments, `reqwest` with rustls so Linux binaries can be static without OpenSSL, `tokio` for concurrent requests, and `serde` for JSON.
- GraphQL queries are `.graphql` files in `graphql/`, compiled into typed Rust by [graphql_client](https://crates.io/crates/graphql_client) and checked against GitHub's published schema, so a misspelled field fails the build. The schema is vendored unmodified; see [graphql/README.md](graphql/README.md) for its provenance and license. Requests go through fue's own client, which reads GitHub's `NOT_FOUND` errors.
- Release builds use link-time optimization, a single code generation unit, and stripped symbols for a smaller and faster binary.
- Tests run the binary against a local [wiremock](https://crates.io/crates/wiremock) mock of the GitHub API selected with `--api-url`, without network or credentials.

## Logo

The logo is a unicorn whose eye is an `@`, a nod to the name: fue is short for Find Unicorn Engineers. It is original artwork, generated as `assets/logo.svg` by `examples/logo.rs`, with the `@` drawn as strokes so it doesn't depend on fonts. Edit the example and run `cargo run --example logo`; `cargo test` fails while the committed SVG differs.

The demo at the top of the README, `assets/demo.gif`, is recorded with [vhs](https://github.com/charmbracelet/vhs) from `assets/demo.tape`, which runs the release build in a real terminal so the spinner shows: run `cargo build --release && vhs assets/demo.tape`, and look at the result before committing it, since it shows live GitHub data.

## Distribution

Releases publish the crate to crates.io with [rust-crate-release](https://github.com/dblock/rust-crate-release) and attach archives with SHA-256 checksums to the GitHub Release: macOS on Apple silicon and Intel, statically linked Linux on arm64 and x86_64, and Windows on x86_64. Every pull request builds the same binaries. A formula in [dblock/homebrew-tap](https://github.com/dblock/homebrew-tap) installs them. See [RELEASING](RELEASING.md).
