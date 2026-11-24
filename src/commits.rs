//! Commits, the addresses in them, and paging through repository history.

use std::collections::BTreeSet;

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
    /// A commit author without a GitHub account, by e-mail.
    Email(String),
}

/// Adds the commits' author addresses, then the `Signed-off-by:` addresses of the same person: sign-offs with the commit author's address or name, or with an address the person used elsewhere, such as on another commit or their profile.
pub fn add_commits(addresses: &mut AddressBook, commits: &[(Source, &Commit)]) {
    for (source, commit) in commits {
        if let Some(email) = &commit.author_email {
            addresses.add(email, commit.author_name.as_deref(), *source, used(commit));
        }
    }
    for (_, commit) in commits {
        for (name, email) in signed_off_by(&commit.message) {
            if signed_off_by_author(commit, name, email) || addresses.contains(email) {
                addresses.add(email, name, Source::SignedOffBy, used(commit));
            }
        }
    }
}

/// A commit's SHA and date, so a commit seen by more than one source is counted once.
fn used(commit: &Commit) -> Option<(&str, &str)> {
    Some((commit.oid.as_str(), commit.committed_date.as_str()))
}

/// Whether a sign-off is by the commit's author, with the same address or name. Others, such as reviewers, also sign off on commits.
fn signed_off_by_author(commit: &Commit, name: Option<&str>, email: &str) -> bool {
    let same_address = commit
        .author_email
        .as_deref()
        .is_some_and(|author| author.eq_ignore_ascii_case(email));
    let same_name = match (name, commit.author_name.as_deref()) {
        (Some(name), Some(author)) => same_name(name, author),
        _ => false,
    };
    same_address || same_name
}

/// Whether one name's words all appear in the other, ignoring case and punctuation, such as `Daniel Doubrovkine` and `Daniel (dB.) Doubrovkine`.
pub fn same_name(a: &str, b: &str) -> bool {
    let words = |name: &str| -> BTreeSet<String> {
        name.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let (a, b) = (words(a), words(b));
    !a.is_empty() && !b.is_empty() && (a.is_subset(&b) || b.is_subset(&a))
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
    let author = match author {
        Author::Id(id) => repository_history::CommitAuthor {
            id: Some(id.clone()),
            emails: None,
        },
        Author::Email(email) => repository_history::CommitAuthor {
            id: None,
            emails: Some(vec![email.clone()]),
        },
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

    fn commit(author_name: Option<&str>, author_email: Option<&str>, message: &str) -> Commit {
        Commit {
            oid: "sha".to_string(),
            committed_date: "2026-01-01T00:00:00Z".to_string(),
            message: message.to_string(),
            author_name: author_name.map(str::to_string),
            author_email: author_email.map(str::to_string),
        }
    }

    fn emails(commit: &Commit) -> Vec<String> {
        emails_of(&[commit])
    }

    fn emails_of(commits: &[&Commit]) -> Vec<String> {
        let mut addresses = AddressBook::default();
        let commits: Vec<(Source, &Commit)> = commits
            .iter()
            .map(|commit| (Source::History, *commit))
            .collect();
        add_commits(&mut addresses, &commits);
        addresses
            .into_addresses()
            .into_iter()
            .map(|address| address.email)
            .collect()
    }

    #[test]
    fn matches_names() {
        assert!(same_name("Daniel Doubrovkine", "Daniel (dB.) Doubrovkine"));
        assert!(same_name("the octocat", "The Octocat"));
        assert!(same_name("octocat", "The Octocat"));
        assert!(!same_name("Theo Truong", "Daniel (dB.) Doubrovkine"));
        assert!(!same_name("", "Daniel"));
        assert!(!same_name("...", "..."));
    }

    #[test]
    fn keeps_sign_offs_by_the_author() {
        let commit = commit(
            Some("The Octocat"),
            Some("1+octocat@users.noreply.github.com"),
            "Fix.\n\nSigned-off-by: Octocat <octocat@example.com>\nSigned-off-by: Mona Lisa <mona@example.com>\nSigned-off-by: <1+octocat@users.noreply.github.com>",
        );
        assert_eq!(
            emails(&commit),
            ["octocat@example.com", "1+octocat@users.noreply.github.com"]
        );
    }

    #[test]
    fn keeps_sign_offs_with_an_address_the_person_used() {
        let signed = commit(
            Some("Daniel (dB.) Doubrovkine"),
            Some("daniel@example.com"),
            "Fix.\n\nSigned-off-by: dblock <work@example.com>\nSigned-off-by: Theo <theo@example.com>",
        );
        let authored = commit(Some("dblock"), Some("work@example.com"), "Work.");
        assert_eq!(
            emails_of(&[&signed, &authored]),
            ["daniel@example.com", "work@example.com"]
        );
        assert_eq!(emails(&signed), ["daniel@example.com"]);
    }

    #[test]
    fn ignores_sign_offs_without_an_author() {
        let commit = commit(
            None,
            None,
            "Fix.\n\nSigned-off-by: The Octocat <octocat@example.com>",
        );
        assert!(emails(&commit).is_empty());
    }

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
