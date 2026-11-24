//! Parses the users and repositories given on the command line.

use std::fmt;
use std::str::FromStr;

/// A GitHub user or repository to search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A GitHub user, such as `dblock`.
    User(String),
    /// A GitHub repository, such as `dblock/fue`.
    Repository { owner: String, name: String },
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::User(login) => write!(f, "{login}"),
            Target::Repository { owner, name } => write!(f, "{owner}/{name}"),
        }
    }
}

impl FromStr for Target {
    type Err = String;

    /// Accepts `login`, `@login`, `owner/repo`, and `https://github.com/...` URLs.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let value = input.trim();
        let value = ["https://github.com/", "http://github.com/", "github.com/"]
            .iter()
            .find_map(|prefix| value.strip_prefix(prefix))
            .unwrap_or(value);
        let value = value.trim_start_matches('@').trim_end_matches('/');
        let value = value.strip_suffix(".git").unwrap_or(value);
        match value.split('/').collect::<Vec<_>>().as_slice() {
            [login] if is_login(login) => Ok(Target::User((*login).to_string())),
            [owner, name] if is_login(owner) && is_repository_name(name) => {
                Ok(Target::Repository {
                    owner: (*owner).to_string(),
                    name: (*name).to_string(),
                })
            }
            _ => Err(format!(
                "`{input}` is not a GitHub user or repository. Use a user such as `dblock` or a repository such as `dblock/fue`."
            )),
        }
    }
}

/// GitHub logins have at most 39 alphanumeric characters or single hyphens, and do not start with a hyphen.
fn is_login(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 39
        && !value.starts_with('-')
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn is_repository_name(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(login: &str) -> Target {
        Target::User(login.to_string())
    }

    fn repository(owner: &str, name: &str) -> Target {
        Target::Repository {
            owner: owner.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn parses_users() {
        assert_eq!("dblock".parse(), Ok(user("dblock")));
        assert_eq!("@dblock".parse(), Ok(user("dblock")));
        assert_eq!(" a-b-c ".parse(), Ok(user("a-b-c")));
        assert_eq!("https://github.com/dblock/".parse(), Ok(user("dblock")));
    }

    #[test]
    fn parses_repositories() {
        assert_eq!("dblock/fue".parse(), Ok(repository("dblock", "fue")));
        assert_eq!("dblock/fue.rs".parse(), Ok(repository("dblock", "fue.rs")));
        assert_eq!(
            "https://github.com/dblock/fue-rs.git".parse(),
            Ok(repository("dblock", "fue-rs"))
        );
        assert_eq!(
            "github.com/dblock/my_repo".parse(),
            Ok(repository("dblock", "my_repo"))
        );
    }

    #[test]
    fn rejects_invalid_targets() {
        for input in [
            "",
            "-dblock",
            "/fue",
            "a/b/c",
            "dbl ock",
            "dblock/..",
            &"a".repeat(40),
        ] {
            let error = input.parse::<Target>().unwrap_err();
            assert!(
                error.contains("is not a GitHub user or repository."),
                "{input}: {error}"
            );
        }
    }

    #[test]
    fn displays_targets() {
        assert_eq!(user("dblock").to_string(), "dblock");
        assert_eq!(repository("dblock", "fue").to_string(), "dblock/fue");
    }
}
