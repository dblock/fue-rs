//! Runs the `fue` binary against a mock GitHub API.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use wiremock::matchers::{
    body_partial_json, body_string_contains, header, method, path, query_param,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

// GraphQL operation names.
const OWNED: &str = "OwnedRepositories";
const CONTRIBUTED: &str = "ContributedRepositories";
const ACTIVITY: &str = "PushHistory";
const MORE_HISTORY: &str = "RepositoryHistory";

/// `fue` with no token in the environment and `gh` out of reach.
fn fue_without_token() -> Command {
    let mut command = assert_cmd::cargo::cargo_bin_cmd!("fue");
    command
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env("PATH", "");
    command
}

fn fue(server: &MockServer) -> Command {
    let mut command = fue_without_token();
    command.args(["--api-url", &server.uri(), "--token", "test-token"]);
    command
}

fn graphql(data: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
}

fn no_repositories() -> Value {
    json!({ "user": { "repositories": { "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [] } } })
}

fn commit(oid: &str, date: &str, name: &str, email: &str, message: &str) -> Value {
    json!({ "oid": oid, "committedDate": date, "message": message, "author": { "name": name, "email": email } })
}

fn history(commits: Vec<Value>, next: Option<&str>) -> Value {
    json!({ "nodes": commits, "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next } })
}

fn repository(name: &str, history: Value) -> Value {
    json!({ "nameWithOwner": name, "defaultBranchRef": { "target": { "__typename": "Commit", "history": history } } })
}

fn repositories(nodes: Vec<Value>, next: Option<&str>) -> Value {
    json!({ "user": { "repositories": { "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next }, "nodes": nodes } } })
}

/// A user with a profile, and lower-priority empty activity and history that tests override.
async fn mock_user(server: &MockServer, login: &str, profile: Value) {
    let mut profile = profile;
    profile["login"] = json!(login);
    profile["node_id"] = json!(format!("U_{login}"));
    Mock::given(method("GET"))
        .and(path(format!("/users/{login}")))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(profile))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/users/{login}/events/public")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .with_priority(10)
        .mount(server)
        .await;
    for connection in [OWNED, CONTRIBUTED] {
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_string_contains(connection))
            .respond_with(graphql(no_repositories()))
            .with_priority(10)
            .mount(server)
            .await;
    }
}

async fn mock_graphql(server: &MockServer, contains: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(contains))
        .respond_with(response)
        .with_priority(1)
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn finds_a_public_profile_address() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "name": "The Octocat", "email": "octocat@example.com" }),
    )
    .await;
    fue(&server)
        .arg("octocat")
        .assert()
        .success()
        .stdout("@octocat\n  octocat@example.com  The Octocat  public profile\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn shows_users_without_an_address() {
    let server = MockServer::start().await;
    mock_user(&server, "hidden", json!({ "name": null, "email": null })).await;
    fue(&server)
        .arg("@hidden")
        .assert()
        .success()
        .stdout("@hidden\n  (no e-mail found)\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn merges_every_source() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "name": "The Octocat", "email": "octocat@example.com" }),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/users/octocat/events/public"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "type": "PushEvent", "repo": { "name": "octocat/hello" }, "payload": { "head": "abc" } },
            { "type": "WatchEvent", "repo": { "name": "octocat/other" }, "payload": {} },
            { "type": "PushEvent", "repo": { "name": "octocat/hello" }, "payload": { "head": "abc" } }
        ])))
        .with_priority(1)
        .mount(&server)
        .await;
    let recent = commit(
        "c1",
        "2026-11-20T10:00:00Z",
        "The Octocat",
        "octocat@example.com",
        "Change.",
    );
    mock_graphql(
        &server,
        ACTIVITY,
        graphql(
            json!({ "repository": { "object": { "__typename": "Commit", "history": history(vec![recent.clone()], None) } } }),
        ),
    )
    .await;
    mock_graphql(
        &server,
        OWNED,
        graphql(repositories(
            vec![repository(
                "octocat/hello",
                history(
                    vec![
                        recent,
                        commit(
                            "c2",
                            "2024-01-10T00:00:00Z",
                            "octocat",
                            "1+octocat@users.noreply.github.com",
                            "Fix.\n\nSigned-off-by: Mona <mona@example.com>",
                        ),
                    ],
                    None,
                ),
            )],
            None,
        )),
    )
    .await;
    mock_graphql(
        &server,
        CONTRIBUTED,
        graphql(repositories(
            vec![repository(
                "other/project",
                history(
                    vec![
                        json!({ "oid": "c4", "committedDate": "2025-06-01T00:00:00Z", "message": "", "author": { "name": "Unknown", "email": null } }),
                        commit(
                        "c3",
                        "2025-05-01T00:00:00Z",
                        "Octo Worker",
                        "work@example.com",
                        "Work.",
                    )],
                    None,
                ),
            )],
            None,
        )),
    )
    .await;

    fue(&server).arg("octocat").assert().success().stdout(
        "@octocat\n\
         \x20 octocat@example.com                 The Octocat  1 commit, last 2026-11-20\n\
         \x20 work@example.com                    Octo Worker  1 commit, last 2025-05-01\n\
         \x20 mona@example.com                    Mona         1 commit, last 2024-01-10\n\
         \x20 1+octocat@users.noreply.github.com  octocat      1 commit, last 2024-01-10  noreply\n",
    );

    let output = fue(&server).args(["--json", "octocat"]).output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sources: Vec<&Value> = value[0]["people"][0]["addresses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|address| &address["sources"])
        .collect();
    assert_eq!(
        sources,
        [
            &json!(["profile", "activity", "history"]),
            &json!(["history"]),
            &json!(["signed-off-by"]),
            &json!(["history"]),
        ]
    );

    fue(&server)
        .args(["--verbose", "octocat"])
        .assert()
        .success()
        .stdout(predicate::str::contains("via profile, activity, history"))
        .stderr("Using a GitHub token from --token.\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn pages_through_repositories() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(OWNED))
        .and(body_partial_json(
            json!({ "variables": { "breadth": 100, "depth": 1, "cursor": null } }),
        ))
        .respond_with(graphql(repositories(
            vec![repository(
                "octocat/first",
                history(
                    vec![commit("c1", "2026-01-01", "Octo", "first@example.com", "")],
                    None,
                ),
            )],
            Some("r1"),
        )))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(OWNED))
        .and(body_partial_json(
            json!({ "variables": { "breadth": 50, "cursor": "r1" } }),
        ))
        .respond_with(graphql(repositories(
            vec![repository(
                "octocat/second",
                history(
                    vec![commit("c2", "2025-01-01", "Octo", "second@example.com", "")],
                    None,
                ),
            )],
            Some("r2"),
        )))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    fue(&server)
        .args(["--breadth", "150", "octocat"])
        .assert()
        .success()
        .stdout(
            "@octocat\n\
             \x20 first@example.com   Octo  1 commit, last 2026-01-01\n\
             \x20 second@example.com  Octo  1 commit, last 2025-01-01\n",
        );
}

#[tokio::test(flavor = "multi_thread")]
async fn pages_through_commits() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(OWNED))
        .and(body_partial_json(json!({ "variables": { "breadth": 2, "depth": 100, "cursor": null } })))
        .respond_with(graphql(repositories(
            vec![
                repository(
                    "octocat/big",
                    history(vec![commit("c1", "2026-01-01", "Octo", "first@example.com", "")], Some("h1")),
                ),
                json!({ "nameWithOwner": "octocat/tree", "defaultBranchRef": { "target": { "__typename": "Tree" } } }),
                repository(
                    "octocat/small",
                    history(vec![commit("c2", "2025-01-01", "Octo", "second@example.com", "")], Some("h3")),
                ),
            ],
            Some("r1"),
        )))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    let more = |name: &str, depth: usize, cursor: &str, response: ResponseTemplate| {
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_string_contains(MORE_HISTORY))
            .and(body_partial_json(json!({ "variables": { "owner": "octocat", "name": name, "depth": depth, "cursor": cursor } })))
            .respond_with(response)
            .with_priority(1)
            .expect(1)
    };
    more(
        "big",
        100,
        "h1",
        graphql(json!({
            "repository": repository(
                "octocat/big",
                history(vec![commit("c3", "2024-01-01", "Octo", "older@example.com", "")], Some("h2")),
            )
        })),
    )
    .mount(&server)
    .await;
    more(
        "big",
        50,
        "h2",
        graphql(json!({
            "repository": repository(
                "octocat/big",
                history(vec![commit("c4", "2023-01-01", "Octo", "oldest@example.com", "")], None),
            )
        })),
    )
    .mount(&server)
    .await;
    // The repository is deleted while paging.
    more("small", 100, "h3", graphql(json!({ "repository": null })))
        .mount(&server)
        .await;
    fue(&server)
        .args(["--breadth", "2", "--depth", "250", "octocat"])
        .assert()
        .success()
        .stdout(
            "@octocat\n\
             \x20 first@example.com   Octo  1 commit, last 2026-01-01\n\
             \x20 second@example.com  Octo  1 commit, last 2025-01-01\n\
             \x20 older@example.com   Octo  1 commit, last 2024-01-01\n\
             \x20 oldest@example.com  Octo  1 commit, last 2023-01-01\n",
        );
}

/// Variables of the GraphQL requests named `operation` that the server received.
async fn graphql_variables(server: &MockServer, operation: &str) -> Vec<Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter_map(|request| serde_json::from_slice::<Value>(&request.body).ok())
        .filter(|body| body["operationName"] == operation)
        .map(|body| body["variables"].clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn requests_fewer_repositories_when_looking_deeper() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    fue(&server)
        .args(["--breadth", "100", "--depth", "10", "octocat"])
        .assert()
        .success();
    for operation in [OWNED, CONTRIBUTED] {
        let variables = graphql_variables(&server, operation).await;
        assert_eq!(variables.len(), 1, "{operation}");
        assert_eq!(variables[0]["breadth"], 20, "{operation}");
        assert_eq!(variables[0]["depth"], 10, "{operation}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn retries_graphql_timeouts_once() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "email": "octocat@example.com" }),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(OWNED))
        .respond_with(ResponseTemplate::new(502))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    fue(&server)
        .arg("octocat")
        .assert()
        .success()
        .stdout("@octocat\n  octocat@example.com  public profile\n");
    assert_eq!(graphql_variables(&server, OWNED).await.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_repeated_graphql_timeouts() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    mock_graphql(&server, OWNED, ResponseTemplate::new(502)).await;
    fue(&server)
        .arg("octocat")
        .assert()
        .failure()
        .stderr("error: octocat: GitHub API returned 502 Bad Gateway for /graphql.\n");
    assert_eq!(graphql_variables(&server, OWNED).await.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn tolerates_missing_repositories_in_activity() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "email": "octocat@example.com" }),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/users/octocat/events/public"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "type": "PushEvent", "repo": { "name": "octocat/deleted" }, "payload": { "head": "abc" } }
        ])))
        .with_priority(1)
        .mount(&server)
        .await;
    mock_graphql(
        &server,
        ACTIVITY,
        ResponseTemplate::new(200).set_body_json(json!({
            "data": { "repository": null },
            "errors": [{ "type": "NOT_FOUND", "message": "Could not resolve to a Repository with the name 'octocat/deleted'." }]
        })),
    )
    .await;
    fue(&server)
        .arg("octocat")
        .assert()
        .success()
        .stdout("@octocat\n  octocat@example.com  public profile\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn looks_up_at_most_ten_recent_pushes() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    let mut events: Vec<Value> =
        vec![json!({ "type": "PushEvent", "repo": { "name": "octocat/hello" }, "payload": {} })];
    events.extend((0..12).map(|index| json!({ "type": "PushEvent", "repo": { "name": "octocat/hello" }, "payload": { "head": format!("head{index}") } })));
    Mock::given(method("GET"))
        .and(path("/users/octocat/events/public"))
        .respond_with(ResponseTemplate::new(200).set_body_json(events))
        .with_priority(1)
        .mount(&server)
        .await;
    mock_graphql(&server, ACTIVITY, graphql(json!({ "repository": null }))).await;
    fue(&server).arg("octocat").assert().success();
    let requests = server.received_requests().await.unwrap();
    let mut heads: Vec<String> = requests
        .iter()
        .filter_map(|request| serde_json::from_slice::<Value>(&request.body).ok())
        .filter(|body| body["operationName"] == ACTIVITY)
        .map(|body| body["variables"]["head"].as_str().unwrap().to_string())
        .collect();
    heads.sort();
    let mut expected: Vec<String> = (0..10).map(|index| format!("head{index}")).collect();
    expected.sort();
    assert_eq!(heads, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_graphql_responses_without_data() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    mock_graphql(
        &server,
        OWNED,
        ResponseTemplate::new(200).set_body_json(json!({ "data": null })),
    )
    .await;
    fue(&server)
        .arg("octocat")
        .assert()
        .failure()
        .stderr("error: octocat: The GitHub GraphQL response has no data.\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_graphql_errors() {
    let server = MockServer::start().await;
    mock_user(&server, "octocat", json!({ "email": null })).await;
    mock_graphql(
        &server,
        CONTRIBUTED,
        ResponseTemplate::new(200).set_body_json(json!({
            "data": null,
            "errors": [{ "type": "FORBIDDEN", "message": "Resource not accessible" }]
        })),
    )
    .await;
    fue(&server)
        .arg("octocat")
        .assert()
        .failure()
        .stdout("")
        .stderr("error: octocat: GitHub GraphQL error: Resource not accessible.\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_unknown_users_and_continues() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "email": "octocat@example.com" }),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/users/nobody"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({ "message": "Not Found" })))
        .mount(&server)
        .await;
    fue(&server)
        .args(["nobody", "octocat"])
        .assert()
        .failure()
        .stdout("@octocat\n  octocat@example.com  public profile\n")
        .stderr("error: nobody: GitHub user `nobody` not found.\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_api_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/octocat"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({ "message": "Bad credentials" })),
        )
        .mount(&server)
        .await;
    fue(&server).arg("octocat").assert().failure().stderr(
        "error: octocat: GitHub API returned 401 Unauthorized for /users/octocat: Bad credentials.\n",
    );
}

#[test]
fn reports_unreachable_apis() {
    fue_without_token()
        .args([
            "--api-url",
            "http://127.0.0.1:9",
            "--token",
            "test-token",
            "octocat",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::starts_with(
            "error: octocat: Failed to reach the GitHub API at http://127.0.0.1:9: ",
        ))
        .stderr(predicate::str::ends_with(".\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn prints_json() {
    let server = MockServer::start().await;
    mock_user(
        &server,
        "octocat",
        json!({ "name": "The Octocat", "email": "octocat@example.com" }),
    )
    .await;
    let output = fue(&server).args(["--json", "octocat"]).output().unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value,
        json!([{
            "target": "octocat",
            "people": [{
                "login": "octocat",
                "name": "The Octocat",
                "addresses": [{
                    "email": "octocat@example.com",
                    "names": ["The Octocat"],
                    "commits": 0,
                    "last_used": null,
                    "noreply": false,
                    "sources": ["profile"]
                }]
            }]
        }])
    );
}

/// Contributors of `octocat/hello`, from the contributors API.
async fn mock_contributors(server: &MockServer, page: &str, contributors: Value) {
    Mock::given(method("GET"))
        .and(path("/repos/octocat/hello/contributors"))
        .and(query_param("anon", "1"))
        .and(query_param("page", page))
        .respond_with(ResponseTemplate::new(200).set_body_json(contributors))
        .mount(server)
        .await;
}

/// The history of `octocat/hello` for one author, a GraphQL `CommitAuthor`.
async fn mock_contributor_history(server: &MockServer, author: Value, commits: Vec<Value>) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(MORE_HISTORY))
        .and(body_partial_json(json!({ "variables": { "owner": "octocat", "name": "hello", "author": author, "cursor": null } })))
        .respond_with(graphql(json!({ "repository": repository("octocat/hello", history(commits, None)) })))
        .with_priority(1)
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn searches_repository_contributors() {
    let server = MockServer::start().await;
    mock_contributors(
        &server,
        "1",
        json!([
            { "type": "User", "login": "octocat", "node_id": "U_octocat", "contributions": 12 },
            { "type": "Anonymous", "email": "anon@example.com", "name": "Anon Person", "contributions": 3 },
            { "type": "Anonymous", "email": "lost@example.com", "name": "Lost Person", "contributions": 1 },
            { "type": "Anonymous", "contributions": 1 }
        ]),
    )
    .await;
    mock_contributor_history(
        &server,
        json!({ "id": "U_octocat" }),
        vec![
            commit(
                "c1",
                "2026-11-20T10:00:00Z",
                "The Octocat",
                "octocat@example.com",
                "Change.\n\nSigned-off-by: Mona <mona@example.com>",
            ),
            commit(
                "c2",
                "2024-01-10T00:00:00Z",
                "octocat",
                "1+octocat@users.noreply.github.com",
                "Fix.",
            ),
        ],
    )
    .await;
    mock_contributor_history(
        &server,
        json!({ "emails": ["anon@example.com"] }),
        vec![commit(
            "c3",
            "2025-05-01T00:00:00Z",
            "Anon Person",
            "anon@example.com",
            "Work.",
        )],
    )
    .await;
    mock_contributor_history(&server, json!({ "emails": ["lost@example.com"] }), vec![]).await;
    fue(&server).arg("octocat/hello").assert().success().stdout(
        "@octocat\n\
         \x20 mona@example.com                    Mona         1 commit, last 2026-11-20\n\
         \x20 octocat@example.com                 The Octocat  1 commit, last 2026-11-20\n\
         \x20 1+octocat@users.noreply.github.com  octocat      1 commit, last 2024-01-10  noreply\n\
         \n\
         Anon Person (no GitHub account)\n\
         \x20 anon@example.com  Anon Person  1 commit, last 2025-05-01\n\
         \n\
         Lost Person (no GitHub account)\n\
         \x20 lost@example.com  Lost Person\n\
         \n\
         (unknown)\n\
         \x20 (no e-mail found)\n",
    );

    let output = fue(&server)
        .args(["--json", "octocat/hello"])
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value[0]["target"], "octocat/hello");
    assert_eq!(value[0]["people"].as_array().unwrap().len(), 4);
    assert_eq!(value[0]["people"][1]["login"], Value::Null);
    assert_eq!(value[0]["people"][1]["name"], "Anon Person");
}

#[tokio::test(flavor = "multi_thread")]
async fn looks_through_100_commits_per_contributor_by_default() {
    let server = MockServer::start().await;
    mock_contributors(
        &server,
        "1",
        json!([{ "type": "User", "login": "octocat", "node_id": "U_octocat", "contributions": 1 }]),
    )
    .await;
    for depth in [100, 5] {
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_string_contains(MORE_HISTORY))
            .and(body_partial_json(
                json!({ "variables": { "depth": depth } }),
            ))
            .respond_with(graphql(json!({ "repository": null })))
            .expect(1)
            .mount(&server)
            .await;
    }
    fue(&server).arg("octocat/hello").assert().success();
    fue(&server)
        .args(["--depth", "5", "octocat/hello"])
        .assert()
        .success();
}

#[tokio::test(flavor = "multi_thread")]
async fn limits_and_pages_through_contributors() {
    let server = MockServer::start().await;
    let users = |range: std::ops::Range<usize>| -> Value {
        range
            .map(|index| json!({ "type": "User", "login": format!("user{index}"), "node_id": format!("U_{index}"), "contributions": 1 }))
            .collect()
    };
    mock_contributors(&server, "1", users(0..100)).await;
    mock_contributors(&server, "2", users(100..103)).await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(MORE_HISTORY))
        .respond_with(graphql(json!({ "repository": null })))
        .mount(&server)
        .await;
    let headings = |limit: &str| -> usize {
        let output = fue(&server)
            .args(["--limit", limit, "octocat/hello"])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with('@'))
            .count()
    };
    assert_eq!(headings("2"), 2);
    assert_eq!(headings("101"), 101);
    assert_eq!(headings("500"), 103);
}

#[tokio::test(flavor = "multi_thread")]
async fn shows_nobody_for_empty_repositories() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/empty/contributors"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    fue(&server)
        .arg("octocat/empty")
        .assert()
        .success()
        .stdout("");
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_unknown_repositories() {
    let server = MockServer::start().await;
    fue(&server)
        .arg("octocat/missing")
        .assert()
        .failure()
        .stderr("error: octocat/missing: GitHub repository `octocat/missing` not found.\n");
}

#[test]
fn rejects_invalid_targets() {
    fue_without_token()
        .arg("not a user")
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "`not a user` is not a GitHub user or repository.",
        ));
}

#[test]
fn rejects_zero_limits() {
    fue_without_token()
        .args(["--depth", "0", "octocat"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--depth"));
}

#[test]
fn requires_a_target() {
    fue_without_token()
        .assert()
        .code(2)
        .stderr(predicate::str::contains("<USER|OWNER/REPO>"));
}

#[test]
fn prints_the_version() {
    fue_without_token()
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("fue {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn prints_help() {
    fue_without_token()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Find e-mail addresses of GitHub users and repository contributors.",
        ))
        .stdout(predicate::str::contains(
            "Contributors to show for a repository, most commits first.",
        ));
}

#[test]
fn explains_how_to_authenticate() {
    fue_without_token()
        .arg("octocat")
        .assert()
        .failure()
        .stdout("")
        .stderr(
            "error: No GitHub token found. Run `gh auth login`, or pass --token or set GH_TOKEN.\n",
        );
}

#[tokio::test(flavor = "multi_thread")]
async fn shows_the_token_source_when_verbose() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/octocat"))
        .and(header("authorization", "Bearer env-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "login": "octocat", "node_id": "U_octocat" })),
        )
        .mount(&server)
        .await;
    for connection in [OWNED, CONTRIBUTED] {
        mock_graphql(&server, connection, graphql(no_repositories())).await;
    }
    fue_without_token()
        .args(["--api-url", &server.uri(), "-v", "octocat"])
        .env("GITHUB_TOKEN", "env-token")
        .assert()
        .success()
        .stderr("Using a GitHub token from GITHUB_TOKEN.\n");
}

/// A `gh` that is installed but not logged in.
#[cfg(unix)]
#[test]
fn explains_how_to_authenticate_when_gh_is_logged_out() {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!("fue-test-gh-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let gh = directory.join("gh");
    std::fs::write(
        &gh,
        "#!/bin/sh\necho 'no oauth token found for github.com' >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    fue_without_token()
        .arg("octocat")
        .env("PATH", &directory)
        .assert()
        .failure()
        .stderr(
            "error: No GitHub token found. Run `gh auth login`, or pass --token or set GH_TOKEN.\n",
        );
    std::fs::remove_dir_all(directory).unwrap();
}

/// A `gh` that only has a token for the mock API's host.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn uses_the_gh_token_for_the_api_host() {
    use std::os::unix::fs::PermissionsExt;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/octocat"))
        .and(header("authorization", "Bearer gh-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "login": "octocat", "node_id": "U_octocat" })),
        )
        .mount(&server)
        .await;
    for connection in [OWNED, CONTRIBUTED] {
        mock_graphql(&server, connection, graphql(no_repositories())).await;
    }
    let host = server.address().to_string();
    let directory = std::env::temp_dir().join(format!("fue-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let gh = directory.join("gh");
    std::fs::write(
        &gh,
        format!("#!/bin/sh\n[ \"$*\" = \"auth token --hostname {host}\" ] && echo gh-token && exit 0\nexit 1\n"),
    )
    .unwrap();
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    fue_without_token()
        .args(["--api-url", &server.uri(), "--verbose", "octocat"])
        .env("PATH", &directory)
        .assert()
        .success()
        .stderr("Using a GitHub token from gh auth token.\n");
    std::fs::remove_dir_all(directory).unwrap();
}
