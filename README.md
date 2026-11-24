# fue

[![Test](https://github.com/dblock/fue-rs/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/test.yml) [![Lint](https://github.com/dblock/fue-rs/actions/workflows/lint.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/lint.yml) [![Package](https://github.com/dblock/fue-rs/actions/workflows/package.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/package.yml) [![Coverage Status](https://coveralls.io/repos/github/dblock/fue-rs/badge.svg?branch=master)](https://coveralls.io/github/dblock/fue-rs?branch=master)

Find e-mail addresses of GitHub users and repository contributors. A Rust rewrite of the [fue](https://github.com/dblock/fue) Ruby gem, built to ship as a single binary for macOS, Linux, and Windows.

## Installation

fue has not been released yet. To build it from source, install [Rust](https://rustup.rs/) and run:

```sh
cargo install --git https://github.com/dblock/fue-rs
```

## Usage

Log in with the [GitHub CLI](https://cli.github.com/) once with `gh auth login`, or set `GH_TOKEN`. Then search one or more users:

```sh
fue octocat
```

```text
@octocat
  octocat@example.com                 The Octocat  12 commits, last 2026-11-20
  work@example.com                    Octo Worker  1 commit, last 2025-05-01
  1+octocat@users.noreply.github.com  octocat      3 commits, last 2024-01-10  noreply
```

fue shows every address it finds, real addresses first and most recently used first. `noreply` addresses do not receive e-mail. Searching repositories, such as `fue octocat/hello-world`, is not supported yet.

## Configuration

| Option | Default | Description |
| --- | --- | --- |
| `--depth COMMITS` | `1` | Commits by the user to look through in each repository. |
| `--breadth REPOSITORIES` | `10` | Repositories to look through, most recently pushed first. |
| `--json` | | Print JSON. |
| `-v`, `--verbose` | | Show every name, the sources of each address, and where the token came from. |
| `--token TOKEN` | `GH_TOKEN`, `GITHUB_TOKEN`, or `gh auth token` | GitHub token. |
| `--api-url URL` | `https://api.github.com` | GitHub API, such as `https://ghe.example.com/api/v3` for GitHub Enterprise Server. |

fue reads public data only. See [DESIGN](DESIGN.md) for the sources it searches.

## Contributing

See [CONTRIBUTING](CONTRIBUTING.md), [DESIGN](DESIGN.md), [CHANGELOG](CHANGELOG.md), and [RELEASING](RELEASING.md). Automated contributors start with [AGENTS](AGENTS.md).

## License

[MIT](LICENSE).
