//! Finds a GitHub token.

use std::fmt;
use std::process::Command;

use anyhow::{Result, bail};

/// Where a GitHub token came from, shown with `--verbose`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    Flag,
    GhTokenEnv,
    GithubTokenEnv,
    GhCli,
}

impl fmt::Display for TokenSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TokenSource::Flag => "--token",
            TokenSource::GhTokenEnv => "GH_TOKEN",
            TokenSource::GithubTokenEnv => "GITHUB_TOKEN",
            TokenSource::GhCli => "gh auth token",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub source: TokenSource,
}

pub const NO_TOKEN: &str =
    "No GitHub token found. Run `gh auth login`, or pass --token or set GH_TOKEN.";

/// Finds a token from `--token`, then `GH_TOKEN`, then `GITHUB_TOKEN`, then `gh auth token` for `host`.
pub fn find_token(
    flag: Option<&str>,
    env: impl Fn(&str) -> Option<String>,
    gh: impl Fn(&str) -> Option<String>,
    host: &str,
) -> Result<Token> {
    let candidates = [
        (TokenSource::Flag, flag.map(str::to_string)),
        (TokenSource::GhTokenEnv, env("GH_TOKEN")),
        (TokenSource::GithubTokenEnv, env("GITHUB_TOKEN")),
    ];
    for (source, value) in candidates {
        if let Some(value) = non_empty(value) {
            return Ok(Token { value, source });
        }
    }
    if let Some(value) = non_empty(gh(host)) {
        return Ok(Token {
            value,
            source: TokenSource::GhCli,
        });
    }
    bail!(NO_TOKEN)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Runs `gh auth token --hostname host`, returning `None` when gh is missing or not logged in.
pub fn gh_auth_token(host: &str) -> Option<String> {
    let output = Command::new("gh")
        .args(["auth", "token", "--hostname", host])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    fn no_gh(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn prefers_the_flag() {
        let token = find_token(
            Some("flag"),
            env(&[("GH_TOKEN", "gh")]),
            no_gh,
            "github.com",
        )
        .unwrap();
        assert_eq!(token.value, "flag");
        assert_eq!(token.source, TokenSource::Flag);
    }

    #[test]
    fn prefers_gh_token_over_github_token() {
        let token = find_token(
            None,
            env(&[("GH_TOKEN", "gh"), ("GITHUB_TOKEN", "github")]),
            no_gh,
            "github.com",
        )
        .unwrap();
        assert_eq!(token.value, "gh");
        assert_eq!(token.source, TokenSource::GhTokenEnv);
    }

    #[test]
    fn uses_github_token() {
        let token = find_token(
            None,
            env(&[("GITHUB_TOKEN", "github")]),
            no_gh,
            "github.com",
        )
        .unwrap();
        assert_eq!(token.source, TokenSource::GithubTokenEnv);
    }

    #[test]
    fn skips_empty_values() {
        let token = find_token(
            Some(" "),
            env(&[("GH_TOKEN", "")]),
            |_| Some("cli\r\n".into()),
            "github.com",
        )
        .unwrap();
        assert_eq!(token.value, "cli");
        assert_eq!(token.source, TokenSource::GhCli);
    }

    #[test]
    fn asks_gh_for_the_api_host() {
        let token = find_token(
            None,
            env(&[]),
            |host| (host == "ghe.example.com").then(|| "enterprise".to_string()),
            "ghe.example.com",
        )
        .unwrap();
        assert_eq!(token.value, "enterprise");
    }

    #[test]
    fn explains_how_to_authenticate() {
        let error = find_token(None, env(&[]), no_gh, "github.com").unwrap_err();
        assert_eq!(error.to_string(), NO_TOKEN);
    }

    #[test]
    fn displays_sources() {
        assert_eq!(TokenSource::Flag.to_string(), "--token");
        assert_eq!(TokenSource::GhTokenEnv.to_string(), "GH_TOKEN");
        assert_eq!(TokenSource::GithubTokenEnv.to_string(), "GITHUB_TOKEN");
        assert_eq!(TokenSource::GhCli.to_string(), "gh auth token");
    }
}
