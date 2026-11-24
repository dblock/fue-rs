# Contributing

fue is the work of [many contributors](https://github.com/dblock/fue-rs/graphs/contributors). Read [DESIGN](DESIGN.md) for the intended behavior before changing it.

## Setup and Validation

Install [rustup](https://rustup.rs/); the toolchain pinned in `rust-toolchain.toml` installs automatically. fue must also build with Rust 1.85, its `rust-version`, which CI checks with `cargo +1.85 build --locked`. Install the lint tools once:

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

`rumdl` lints Markdown, including Title Case headings. Dylint runs [dylint-spellcheck](https://github.com/dblock/dylint-spellcheck) on Rust comments and Markdown, with the case-sensitive vocabulary in `.spellcheck.txt`; fix misspellings, and add only legitimate technical terms. Check coverage with:

```sh
cargo llvm-cov --locked --show-missing-lines
```

## Tests

Unit tests live next to the code in `src/`. CLI tests in `tests/cli.rs` run the `fue` binary with [assert_cmd](https://crates.io/crates/assert_cmd) against a [wiremock](https://crates.io/crates/wiremock) mock of the GitHub API, passed with `--api-url`, and match GraphQL requests by operation name, such as `OwnedRepositories`. Tests must not reach the network or need credentials, and fixtures use `example.com` addresses rather than real people's. Add tests for new behavior, including errors and edge cases, and keep coverage close to complete.

## Benchmarks

[Criterion](https://crates.io/crates/criterion) benchmarks live in `benches/`: `search.rs` runs user searches against a mock GitHub API with 100 repositories of 100 commits and 50 ms per request, printing the GraphQL requests each makes, and `addresses.rs` measures address and commit handling. CI runs each benchmark once. Compare a change against `master` with a saved baseline:

```sh
cargo bench --benches -- --save-baseline master
cargo bench --benches -- --baseline master
```

## GraphQL

Queries live in `graphql/*.graphql`, each with the fragments it uses, and are compiled into typed Rust by `graphql_client` derives in `src/graphql.rs`, checked against GitHub's schema in `graphql/github.graphql`. To use a new field, add it to a query and let the compiler show the generated types. Update the schema as described in [graphql/README.md](graphql/README.md).

## Pull Requests

Work on a topic branch named `<github-login>/<topic>`. Keep each PR to one coherent commit; split larger work into stacked PRs, each targeting the branch below it. PRs are merged with rebase merges only, and `master` accepts no force pushes. Update README, DESIGN, and AGENTS when behavior changes, and add a line to [CHANGELOG](CHANGELOG.md) under the Next section, linked to the PR, above `- Your contribution here.`. Use Title Case for Markdown headings and GitHub Actions workflow, job, and step names, and keep each Markdown paragraph on one line. See [RELEASING](RELEASING.md) for releases.
