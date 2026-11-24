# Working on fue

Read [CONTRIBUTING](CONTRIBUTING.md) and [DESIGN](DESIGN.md) first.

## Project

fue is a Rust command-line tool that finds e-mail addresses of GitHub users and repository contributors from public GitHub API data. Non-goals: scraping web pages or other sites, sending e-mail, storing tokens, and correlating linked accounts. The binary is `src/main.rs`; the library modules in `src/` cover arguments (`cli.rs`), targets (`target.rs`), tokens (`auth.rs`), the API client (`github.rs`), typed GraphQL queries (`graphql.rs`, with the queries and GitHub's schema in `graphql/`), commits and history paging (`commits.rs`), addresses (`model.rs`), output (`output.rs`), user search (`user.rs`), and repository search (`repository.rs`). CLI subprocess tests against a mock API are in `tests/cli.rs`. `examples/logo.rs` generates `assets/logo.svg`; never edit the SVG by hand. fue builds with Rust 1.85 (`rust-version`) and is developed and tested on the toolchain pinned in `rust-toolchain.toml`; do not use language features newer than 1.85, such as `let` chains, in `src/`.

## Invariants

- Write results to standard output and diagnostics to standard error, so output can be piped.
- End help text, errors, and other messages with a period.
- Never store, log, or print tokens.
- Tests must not use the network or credentials; point fue at a local mock API with `--api-url`.
- Use `example.com` addresses in tests and documentation, never real people's e-mail.
- Write GraphQL queries as `.graphql` files with typed `graphql_client` derives, never as query strings in Rust.
- Keep `graphql/github.graphql` unmodified, and record its source commit in `graphql/README.md` when updating it.
- Do not weaken lint settings or add broad lint exceptions; scope any exception narrowly with a concrete reason.

## Validation

Run all of these and fix what they report before every commit and push:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
rumdl check .
RUSTFLAGS="-D warnings" cargo dylint --all -- --all-targets
cargo package --locked
```

Check coverage with `cargo llvm-cov --locked --show-missing-lines` and add behavioral tests for uncovered paths rather than trivial assertions. CI runs Test on Linux, macOS, and Windows and on Rust 1.85, plus Lint, Package (including release binaries for every target in `.github/workflows/binaries.yml`), and Coveralls coverage.

## Conventions

- Branch as `dblock/<topic>`, keep one commit per PR, stack dependent PRs and register them with `gh stack link`, and use rebase merges only. Never force-push `master`; update your own PR branches with `--force-with-lease`.
- Add a linked CHANGELOG line under the Next section for each change; predict the PR number from the repository's latest issue or PR and verify it after opening the PR.
- Use Title Case for Markdown headings and workflow, job, and step names; keep Markdown paragraphs on one line.
- Prefer current stable releases of dependencies and GitHub Actions, keeping the Rust 1.85 minimum unless a documented change raises it.
- Include an AI co-author trailer in AI-assisted commits, and disclose AI authorship in PR descriptions and comments.
- Never publish to crates.io, push tags, create releases, or change credentials or trusted publishers without explicit maintainer authorization. See [RELEASING](RELEASING.md).
