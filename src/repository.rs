//! Finds the e-mail addresses contributors used in a GitHub repository.

use anyhow::{Result, anyhow};
use futures::stream::{self, StreamExt, TryStreamExt};
use serde::Deserialize;

use crate::commits::{self, Author, PAGE};
use crate::github::GitHub;
use crate::model::{AddressBook, Person, Source};

/// How much of a repository to look through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Commits by each contributor.
    pub depth: usize,
    /// Contributors, most commits first.
    pub limit: usize,
}

/// Contributors searched at the same time.
const CONCURRENCY: usize = 8;

/// A GitHub user, bot, or commit author without a GitHub account, from the contributors API.
#[derive(Debug, Deserialize)]
struct Contributor {
    login: Option<String>,
    node_id: Option<String>,
    email: Option<String>,
    name: Option<String>,
}

/// Searches the commits of a repository's top contributors, in order of commits.
pub async fn search(
    github: &GitHub,
    owner: &str,
    name: &str,
    limits: Limits,
) -> Result<Vec<Person>> {
    let contributors = contributors(github, owner, name, limits.limit).await?;
    stream::iter(contributors)
        .map(|contributor| person(github, owner, name, contributor, limits.depth))
        .buffered(CONCURRENCY)
        .try_collect()
        .await
}

/// Up to `limit` contributors, including commit authors without a GitHub account.
async fn contributors(
    github: &GitHub,
    owner: &str,
    name: &str,
    limit: usize,
) -> Result<Vec<Contributor>> {
    let mut contributors = Vec::new();
    for page in 1.. {
        let batch: Vec<Contributor> = github
            .get_list(&format!(
                "/repos/{owner}/{name}/contributors?anon=1&per_page={PAGE}&page={page}"
            ))
            .await?
            .ok_or_else(|| anyhow!("GitHub repository `{owner}/{name}` not found."))?;
        let last = batch.len() < PAGE;
        contributors.extend(batch);
        if last || contributors.len() >= limit {
            break;
        }
    }
    contributors.truncate(limit);
    Ok(contributors)
}

/// The addresses a contributor used in the repository's history.
async fn person(
    github: &GitHub,
    owner: &str,
    name: &str,
    contributor: Contributor,
    depth: usize,
) -> Result<Person> {
    let mut addresses = AddressBook::default();
    let author = match (&contributor.node_id, &contributor.email) {
        (Some(id), _) => Some(Author::Id(id.clone())),
        (None, Some(email)) => {
            // The contributors API reports this address even when its commits are not found.
            addresses.add(email, contributor.name.as_deref(), Source::History, None);
            Some(Author::Email(email.clone()))
        }
        (None, None) => None,
    };
    if let Some(author) = author {
        for commit in commits::repository_history(github, owner, name, &author, None, depth).await?
        {
            commits::add_commit(&mut addresses, &commit, Source::History);
        }
    }
    let login = contributor.login;
    Ok(Person {
        name: if login.is_some() {
            None
        } else {
            contributor.name
        },
        login,
        addresses: addresses.into_addresses(),
    })
}
