<img src="assets/logo.svg" alt="A white unicorn in profile with a rainbow mane and an @ for an eye." width="160"> <!-- rumdl-disable-line MD033 MD041 -->

# fue

[![Test](https://github.com/dblock/fue-rs/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/test.yml) [![Lint](https://github.com/dblock/fue-rs/actions/workflows/lint.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/lint.yml) [![Package](https://github.com/dblock/fue-rs/actions/workflows/package.yml/badge.svg?branch=master)](https://github.com/dblock/fue-rs/actions/workflows/package.yml) [![Coverage Status](https://coveralls.io/repos/github/dblock/fue-rs/badge.svg?branch=master)](https://coveralls.io/github/dblock/fue-rs?branch=master)

fue, short for Find Unicorn Engineers, finds the e-mail addresses of a GitHub user, or of a repository's contributors, from their public commits. It's a Rust port of the [fue](https://github.com/dblock/fue) Ruby tool, introduced in [Finding a GitHub User's E-Mail Address](https://code.dblock.org/2018/08/15/finding-a-github-users-email-address.html).

![fue finding the e-mail address of the GitHub user defunkt, with a spinner while it searches.](assets/demo.gif)

## Installation

```sh
brew install dblock/tap/fue
```

Or download a binary for macOS, Linux, or Windows from [Releases](https://github.com/dblock/fue-rs/releases), or run `cargo install fue`.

## Usage

Log in once with the [GitHub CLI](https://cli.github.com/), then pass a user or a repository:

```sh
gh auth login
fue octocat
fue octocat/hello-world
```

```text
@octocat
  octocat@example.com                 The Octocat  12 commits, last 2026-11-20
  1+octocat@users.noreply.github.com  octocat      3 commits, last 2024-01-10  noreply
```

Real addresses come first, most recently used first. `noreply` addresses don't receive e-mail.

## Configuration

Most searches need no options. Run `fue --help` for all of them. The most useful are:

| Option | Description |
| --- | --- |
| `--depth COMMITS` | Read more commits in each repository (default 1 for a user, 100 for each contributor). |
| `--breadth REPOSITORIES` | Read more of a user's repositories (default 10). |
| `--limit CONTRIBUTORS` | Show more of a repository's contributors (default 10). |
| `-v`, `--verbose` | Also show every name, where each address was found, and others' `Signed-off-by:` addresses. |
| `--json` | Print JSON. |

fue uses a token from `GH_TOKEN`, `GITHUB_TOKEN`, or `gh auth token`. Pass `--token` to use another one, and `--api-url` for GitHub Enterprise Server.

## Contributing

See [CONTRIBUTING](CONTRIBUTING.md), [DESIGN](DESIGN.md), [CHANGELOG](CHANGELOG.md), and [RELEASING](RELEASING.md). Automated contributors start with [AGENTS](AGENTS.md).

## License

[MIT](LICENSE).
