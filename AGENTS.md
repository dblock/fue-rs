# Working on fue

Read [CONTRIBUTING](CONTRIBUTING.md) and [DESIGN](DESIGN.md) first.

## Project

fue is a Rust command-line tool that finds e-mail addresses of GitHub users and repository contributors from public GitHub API data. Non-goals: scraping web pages or other sites, sending e-mail, storing tokens, and correlating linked accounts.

- `src/main.rs` is the binary. Library modules: `cli.rs` (arguments), `target.rs` (targets), `auth.rs` (tokens), `github.rs` (API client), `graphql.rs` (typed queries, from `graphql/`), `commits.rs` (commits, sign-offs, and history paging), `model.rs` (addresses), `output.rs` (text and JSON), `spinner.rs` (the terminal spinner), `user.rs` and `repository.rs` (searches).
- `tests/cli.rs` runs the binary against a mock API; `benches/` holds benchmarks.
- `examples/logo.rs` generates `assets/logo.svg`; never edit the SVG by hand.
- fue builds with Rust 1.85 (`rust-version`) and is developed on the toolchain in `rust-toolchain.toml`. Do not use language features newer than 1.85, such as `let` chains, in `src/`.

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

Check coverage with `cargo llvm-cov --locked --show-missing-lines`, and add behavioral tests for uncovered paths rather than trivial assertions. Measure performance changes against a benchmark baseline, as described in CONTRIBUTING, and report the numbers.

## Conventions

- Branch as `dblock/<topic>`, keep one commit per PR, stack dependent PRs and register them with `gh stack link`, and use rebase merges only. Never force-push `master`; update your own PR branches with `--force-with-lease`.
- Add a linked CHANGELOG line under the Next section for each change; predict the PR number from the repository's latest issue or PR and verify it after opening the PR.
- Use Title Case for Markdown headings and workflow, job, and step names; keep Markdown paragraphs on one line.
- Prefer current stable releases of dependencies and GitHub Actions, keeping the Rust 1.85 minimum unless a documented change raises it.
- Include an AI co-author trailer in AI-assisted commits, and disclose AI authorship in PR descriptions and comments.
- Never publish to crates.io, push tags, create releases, or change credentials or trusted publishers without explicit maintainer authorization. See [RELEASING](RELEASING.md).
