//! Benchmarks user search end to end against a mock GitHub API with request latency, and reports the requests each search makes.
//!
//! Run with `cargo bench --bench search`.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use fue::commits::SignOffs;
use fue::github::GitHub;
use fue::user::{self, Limits};
use serde_json::{Value, json};
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// Latency of each mock API request, typical of the GitHub API.
const LATENCY: Duration = Duration::from_millis(50);
/// Repositories the mock user owns and contributed to.
const REPOSITORIES: usize = 100;
/// Commits by the mock user in each repository.
const COMMITS: usize = 100;

/// Answers GraphQL queries from their variables, paging through `REPOSITORIES` repositories of `COMMITS` commits each.
struct GraphQl {
    requests: Arc<AtomicUsize>,
}

fn commits(repository: &str, start: usize, count: usize) -> Vec<Value> {
    (start..(start + count).min(COMMITS))
        .map(|index| {
            json!({
                "oid": format!("{repository}-{index}"),
                "committedDate": format!("2026-01-{:02}T00:00:00Z", 1 + index % 28),
                "message": format!("Change {index}.\n\nSigned-off-by: Octo Cat <octocat@example.com>"),
                "author": { "name": "Octo Cat", "email": format!("octocat{}@example.com", index % 3) }
            })
        })
        .collect()
}

fn history(repository: &str, start: usize, depth: usize) -> Value {
    let end = (start + depth).min(COMMITS);
    json!({
        "pageInfo": { "hasNextPage": end < COMMITS, "endCursor": format!("{end}") },
        "nodes": commits(repository, start, depth)
    })
}

fn cursor(variables: &Value) -> usize {
    variables["cursor"]
        .as_str()
        .and_then(|cursor| cursor.parse().ok())
        .unwrap_or(0)
}

impl Respond for GraphQl {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        self.requests.fetch_add(1, Ordering::Relaxed);
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let variables = &body["variables"];
        let depth = variables["depth"].as_u64().unwrap() as usize;
        let data = match body["operationName"].as_str().unwrap() {
            "OwnedRepositories" | "ContributedRepositories" => {
                let start = cursor(variables);
                let end =
                    (start + variables["breadth"].as_u64().unwrap() as usize).min(REPOSITORIES);
                let nodes: Vec<Value> = (start..end)
                    .map(|index| {
                        let name = format!("octocat/repository{index}");
                        json!({
                            "nameWithOwner": name,
                            "defaultBranchRef": { "target": { "__typename": "Commit", "history": history(&name, 0, depth) } }
                        })
                    })
                    .collect();
                json!({ "user": { "repositories": {
                    "pageInfo": { "hasNextPage": end < REPOSITORIES, "endCursor": format!("{end}") },
                    "nodes": nodes
                } } })
            }
            "RepositoryHistory" => {
                let name = format!(
                    "{}/{}",
                    variables["owner"].as_str().unwrap(),
                    variables["name"].as_str().unwrap()
                );
                json!({ "repository": { "defaultBranchRef": { "target": {
                    "__typename": "Commit",
                    "history": history(&name, cursor(variables), depth)
                } } } })
            }
            _ => json!({ "repository": null }),
        };
        ResponseTemplate::new(200)
            .set_body_json(json!({ "data": data }))
            .set_delay(LATENCY)
    }
}

/// A mock GitHub API for the user `octocat`, and its GraphQL request counter.
async fn mock_github() -> (MockServer, Arc<AtomicUsize>) {
    let server = MockServer::start().await;
    let requests = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/users/octocat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "login": "octocat", "name": "Octo Cat", "email": "octocat@example.com", "node_id": "U_octocat" }))
                .set_delay(LATENCY),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/octocat/events/public"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([]))
                .set_delay(LATENCY),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(GraphQl {
            requests: requests.clone(),
        })
        .mount(&server)
        .await;
    (server, requests)
}

fn search(criterion: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let (server, requests) = runtime.block_on(mock_github());
    let github = GitHub::new(&server.uri(), "token").unwrap();
    let mut group = criterion.benchmark_group("user search");
    group.sample_size(10);
    for (breadth, depth) in [(100, 1), (100, 10), (100, 100), (10, 100)] {
        let limits = Limits {
            depth,
            breadth,
            sign_offs: SignOffs::Own,
        };
        requests.store(0, Ordering::Relaxed);
        let person = runtime
            .block_on(user::search(&github, "octocat", limits))
            .unwrap();
        eprintln!(
            "breadth {breadth}, depth {depth}: {} GraphQL requests, {} addresses",
            requests.load(Ordering::Relaxed),
            person.addresses.len()
        );
        group.bench_function(format!("breadth {breadth}, depth {depth}"), |bencher| {
            bencher
                .to_async(&runtime)
                .iter(|| user::search(&github, "octocat", limits));
        });
    }
    group.finish();
}

criterion_group!(benches, search);
criterion_main!(benches);
