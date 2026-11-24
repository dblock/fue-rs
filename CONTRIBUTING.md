# Contributing

fue is the work of [many contributors](https://github.com/dblock/fue-rs/graphs/contributors). Read [DESIGN](DESIGN.md) for the intended behavior before changing it.

## Setup and Validation

Install [rustup](https://rustup.rs/). The pinned toolchain in `rust-toolchain.toml` is installed automatically; the crate also supports Rust 1.85, the `rust-version` in `Cargo.toml`. Install the lint tools once:

```sh
cargo install rumdl --locked
cargo install cargo-dylint dylint-link --version 6.4.0 --locked
cargo install cargo-llvm-cov --locked
```

Run every check before committing and pushing:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
rumdl check .
RUSTFLAGS="-D warnings" cargo dylint --all -- --all-targets
cargo package --locked
```

`rumdl` lints Markdown, including Title Case headings. Dylint runs [dylint-spellcheck](https://github.com/dblock/dylint-spellcheck) on Rust comments and the root Markdown documents, using the reviewed, case-sensitive vocabulary in `.spellcheck.txt`; fix misspellings and add only legitimate technical terms to the vocabulary. Check coverage with:

```sh
cargo llvm-cov --locked --show-missing-lines
```

## Tests

Tests live in `tests/` and run the real `fue` binary as a subprocess with [assert_cmd](https://crates.io/crates/assert_cmd). Tests must not reach the network or need credentials: point fue at a local mock of the GitHub API with `--api-url`, and use `example.com` addresses rather than real people's e-mail in fixtures. Add tests for new behavior, including errors and edge cases, and keep coverage of project code close to complete.

## Pull Requests

Work on a topic branch named `<github-login>/<topic>`. Keep each PR to one coherent commit; split larger work into stacked PRs, each targeting the branch below it. PRs are merged with rebase merges only, and `master` accepts no force pushes. Update README, DESIGN, and AGENTS when behavior changes, and add a line to [CHANGELOG](CHANGELOG.md) under the Next section, linked to the PR, above `- Your contribution here.`. Use Title Case for Markdown headings and GitHub Actions workflow, job, and step names, and keep each Markdown paragraph on one line. See [RELEASING](RELEASING.md) for releases.
