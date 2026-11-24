//! Commits, the addresses in them, and paging through repository history.

use anyhow::Result;

use crate::github::GitHub;
use crate::graphql::{RepositoryHistory, repository_history};
use crate::model::{AddressBook, Source};

/// GitHub returns at most 100 items per page.
pub const PAGE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub oid: String,
    pub committed_date: String,
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// A page of commits and the cursor of the next page.
#[derive(Debug, Default)]
pub struct Page {
    pub commits: Vec<Commit>,
    pub next: Option<String>,
}

/// A page of repositories, each `owner/name` with its first page of commits, and the cursor of the next page.
#[derive(Debug, Default)]
pub struct RepositoryPage {
    pub repositories: Vec<(String, Page)>,
    pub next: Option<String>,
}

/// Whose commits to look for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Author {
    /// A GitHub user, by GraphQL ID.
    Id(String),
}

/// Adds a commit's author and `Signed-off-by:` addresses.
pub fn add_commit(addresses: &mut AddressBook, commit: &Commit, source: Source) {
    let used = Some((commit.oid.as_str(), commit.committed_date.as_str()));
    if let Some(email) = &commit.author_email {
        addresses.add(email, commit.author_name.as_deref(), source, used);
    }
    for (name, email) in signed_off_by(&commit.message) {
        addresses.add(email, name, Source::SignedOffBy, used);
    }
}

/// The names and addresses in `Signed-off-by: Name <address>` lines.
pub fn signed_off_by(message: &str) -> Vec<(Option<&str>, &str)> {
    message
        .lines()
        .filter_map(|line| {
            let value = line.trim().strip_prefix("Signed-off-by:")?.trim();
            let (name, address) = value.rsplit_once('<')?;
            let email = address.strip_suffix('>')?.trim();
            let name = name.trim();
            Some(((!name.is_empty()).then_some(name), email))
        })
        .collect()
}

/// Up to `remaining` default-branch commits by `author` in `owner/name`, starting after `cursor`.
pub async fn repository_history(
    github: &GitHub,
    owner: &str,
    name: &str,
    author: &Author,
    mut cursor: Option<String>,
    mut remaining: usize,
) -> Result<Vec<Commit>> {
    let Author::Id(id) = author;
    let author = repository_history::CommitAuthor {
        id: Some(id.clone()),
        emails: None,
    };
    let mut commits = Vec::new();
    while remaining > 0 {
        let depth = remaining.min(PAGE);
        let variables = repository_history::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
            author: author.clone(),
            depth: depth as i64,
            cursor: cursor.clone(),
        };
        let data = github.query::<RepositoryHistory>(variables).await?;
        let target = data
            .repository
            .and_then(|repository| repository.default_branch_ref)
            .and_then(|branch| branch.target);
        let Some(repository_history::RepositoryHistoryRepositoryDefaultBranchRefTarget::Commit(
            commit,
        )) = target
        else {
            break;
        };
        let page = Page::from(commit.history);
        commits.extend(page.commits);
        remaining -= depth;
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_signed_off_by_lines() {
        let message = "Fix a bug.\n\nSigned-off-by: The Octocat <octocat@example.com>\n  Signed-off-by: <anonymous@example.com>\nSigned-off-by: no address\nsigned-off-by: Lower <lower@example.com>\n";
        assert_eq!(
            signed_off_by(message),
            vec![
                (Some("The Octocat"), "octocat@example.com"),
                (None, "anonymous@example.com"),
            ]
        );
    }
}
