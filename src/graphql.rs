//! Typed GitHub GraphQL queries in `graphql/`, checked against `graphql/github.graphql` when fue compiles.

use graphql_client::GraphQLQuery;

use crate::commits::{Commit, Page, RepositoryPage};

/// GraphQL `DateTime`, an ISO 8601 timestamp.
type DateTime = String;
/// GraphQL `GitObjectID`, a commit SHA.
#[allow(clippy::upper_case_acronyms)]
type GitObjectID = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/github.graphql",
    query_path = "graphql/owned_repositories.graphql",
    response_derives = "Debug"
)]
pub struct OwnedRepositories;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/github.graphql",
    query_path = "graphql/contributed_repositories.graphql",
    response_derives = "Debug"
)]
pub struct ContributedRepositories;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/github.graphql",
    query_path = "graphql/repository_history.graphql",
    response_derives = "Debug",
    variables_derives = "Clone, Debug"
)]
pub struct RepositoryHistory;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/github.graphql",
    query_path = "graphql/push_history.graphql",
    response_derives = "Debug"
)]
pub struct PushHistory;

/// Each query has its own copy of the `History` and `CommitFields` fragments.
macro_rules! history_conversions {
    ($($module:ident),*) => {$(
        impl From<$module::CommitFields> for Commit {
            fn from(commit: $module::CommitFields) -> Self {
                let (author_name, author_email) = commit
                    .author
                    .map(|author| (author.name, author.email))
                    .unwrap_or_default();
                Commit {
                    oid: commit.oid,
                    committed_date: commit.committed_date,
                    message: commit.message,
                    author_name,
                    author_email,
                }
            }
        }

        impl From<$module::History> for Page {
            fn from(history: $module::History) -> Self {
                Page {
                    next: history
                        .page_info
                        .end_cursor
                        .filter(|_| history.page_info.has_next_page),
                    commits: history
                        .nodes
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(Commit::from)
                        .collect(),
                }
            }
        }
    )*};
}

history_conversions!(
    owned_repositories,
    contributed_repositories,
    repository_history,
    push_history
);

macro_rules! repositories_conversions {
    ($($module:ident),*) => {$(
        impl From<$module::Repositories> for RepositoryPage {
            fn from(repositories: $module::Repositories) -> Self {
                RepositoryPage {
                    next: repositories
                        .page_info
                        .end_cursor
                        .filter(|_| repositories.page_info.has_next_page),
                    repositories: repositories
                        .nodes
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter_map(|node| match node.default_branch_ref?.target? {
                            $module::RepositoriesNodesDefaultBranchRefTarget::Commit(commit) => {
                                Some((node.name_with_owner, Page::from(commit.history)))
                            }
                            _ => None,
                        })
                        .collect(),
                }
            }
        }
    )*};
}

repositories_conversions!(owned_repositories, contributed_repositories);
