//! Finds a GitHub user's e-mail addresses.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use futures::future::join_all;
use serde::Deserialize;
use serde_json::Value;

use crate::commits::{self, Author, Commit, PAGE, Page, RepositoryPage};
use crate::github::GitHub;
use crate::graphql::{
    ContributedRepositories, OwnedRepositories, PushHistory, contributed_repositories,
    owned_repositories, push_history,
};
use crate::model::{AddressBook, Person, Source};

/// How much commit history to look through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Commits by the user in each repository.
    pub depth: usize,
    /// Repositories, most recently pushed first.
    pub breadth: usize,
}

/// Recent pushes looked up for the activity source.
const PUSHES: usize = 10;
/// Commits by the user looked up from each recent push.
const PUSH_DEPTH: i64 = 10;

#[derive(Deserialize)]
struct User {
    login: String,
    name: Option<String>,
    email: Option<String>,
    node_id: String,
}

/// Searches a user's profile, recent activity, and commit history.
pub async fn search(github: &GitHub, login: &str, limits: Limits) -> Result<Person> {
    let user: User = github
        .get(&format!("/users/{login}"))
        .await?
        .ok_or_else(|| anyhow!("GitHub user `{login}` not found."))?;
    let mut addresses = AddressBook::default();
    if let Some(email) = &user.email {
        addresses.add(email, user.name.as_deref(), Source::Profile, None);
    }
    let (activity, owned, contributed) = tokio::join!(
        activity(github, &user),
        history(github, &user, Connection::Owned, limits),
        history(github, &user, Connection::Contributed, limits),
    );
    for (source, commits) in [
        (Source::Activity, activity?),
        (Source::History, owned?),
        (Source::History, contributed?),
    ] {
        for commit in &commits {
            commits::add_commit(&mut addresses, commit, source);
        }
    }
    Ok(Person {
        login: Some(user.login),
        name: user.name,
        addresses: addresses.into_addresses(),
    })
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    kind: String,
    repo: EventRepository,
    payload: Value,
}

#[derive(Deserialize)]
struct EventRepository {
    name: String,
}

/// The user's commits leading up to their recent pushes. Push events no longer list commits, so each push's head is looked up.
async fn activity(github: &GitHub, user: &User) -> Result<Vec<Commit>> {
    let events: Vec<Event> = github
        .get(&format!("/users/{}/events/public?per_page=100", user.login))
        .await?
        .unwrap_or_default();
    let mut heads = BTreeSet::new();
    let mut pushes = Vec::new();
    for event in events.iter().filter(|event| event.kind == "PushEvent") {
        let head = event.payload.get("head").and_then(Value::as_str);
        if let (Some(head), Some((owner, name))) = (head, event.repo.name.split_once('/')) {
            if heads.insert(head) {
                pushes.push((owner, name, head));
            }
        }
        if pushes.len() == PUSHES {
            break;
        }
    }
    let results = join_all(pushes.into_iter().map(|(owner, name, head)| {
        github.query::<PushHistory>(push_history::Variables {
            owner: owner.to_string(),
            name: name.to_string(),
            head: head.to_string(),
            author_id: user.node_id.clone(),
            depth: PUSH_DEPTH,
        })
    }))
    .await;
    let mut commits = Vec::new();
    for result in results {
        // A repository deleted since the push is `null`.
        if let Some(push_history::PushHistoryRepositoryObject::Commit(commit)) =
            result?.repository.and_then(|repository| repository.object)
        {
            commits.extend(Page::from(commit.history).commits);
        }
    }
    Ok(commits)
}

/// Repositories the user owns, including forks, or has contributed commits to.
#[derive(Debug, Clone, Copy)]
enum Connection {
    Owned,
    Contributed,
}

async fn repositories(
    github: &GitHub,
    user: &User,
    connection: Connection,
    depth: i64,
    breadth: i64,
    cursor: Option<String>,
) -> Result<Option<RepositoryPage>> {
    Ok(match connection {
        Connection::Owned => github
            .query::<OwnedRepositories>(owned_repositories::Variables {
                login: user.login.clone(),
                author_id: user.node_id.clone(),
                depth,
                breadth,
                cursor,
            })
            .await?
            .user
            .map(|user| user.repositories.into()),
        Connection::Contributed => github
            .query::<ContributedRepositories>(contributed_repositories::Variables {
                login: user.login.clone(),
                author_id: user.node_id.clone(),
                depth,
                breadth,
                cursor,
            })
            .await?
            .user
            .map(|user| user.repositories.into()),
    })
}

/// The user's commits on the default branch of each repository, up to `limits`.
async fn history(
    github: &GitHub,
    user: &User,
    connection: Connection,
    limits: Limits,
) -> Result<Vec<Commit>> {
    let author = Author::Id(user.node_id.clone());
    let depth = limits.depth.min(PAGE) as i64;
    let mut commits = Vec::new();
    let mut cursor: Option<String> = None;
    let mut remaining = limits.breadth;
    while remaining > 0 {
        let breadth = remaining.min(PAGE);
        let Some(page) =
            repositories(github, user, connection, depth, breadth as i64, cursor).await?
        else {
            break;
        };
        for (name_with_owner, history) in page.repositories {
            commits.extend(history.commits);
            // GitHub always returns `owner/name`; this guards against malformed responses.
            let repository = name_with_owner.split_once('/');
            if let (Some(next), Some((owner, name)), true) =
                (history.next, repository, limits.depth > PAGE)
            {
                commits.extend(
                    commits::repository_history(
                        github,
                        owner,
                        name,
                        &author,
                        Some(next),
                        limits.depth - PAGE,
                    )
                    .await?,
                );
            }
        }
        remaining -= breadth;
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    Ok(commits)
}
