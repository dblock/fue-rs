//! A small GitHub REST and GraphQL client.

use anyhow::{Context, Result, anyhow, bail};
use graphql_client::GraphQLQuery;
use reqwest::{StatusCode, Url, header};
use serde::Deserialize;
use serde::de::DeserializeOwned;

pub const DEFAULT_API_URL: &str = "https://api.github.com";

pub struct GitHub {
    http: reqwest::Client,
    api_url: String,
    graphql_url: String,
}

impl GitHub {
    pub fn new(api_url: &str, token: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {token}"))
            .context("The GitHub token contains invalid characters")?;
        authorization.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, authorization);
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static("2022-11-28"),
        );
        let http = reqwest::Client::builder()
            .user_agent(concat!("fue/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()?;
        let api_url = api_url.trim_end_matches('/').to_string();
        Ok(GitHub {
            http,
            graphql_url: graphql_url(&api_url),
            api_url,
        })
    }

    /// Gets a REST resource, returning `None` when it does not exist.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let response = self
            .http
            .get(format!("{}{path}", self.api_url))
            .send()
            .await
            .with_context(|| format!("Failed to reach the GitHub API at {}", self.api_url))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = check(response, path).await?;
        Ok(Some(response.json().await?))
    }

    /// Runs a typed GraphQL query and returns its `data`. `NOT_FOUND` errors are tolerated when the response has data, leaving the missing objects `null`.
    pub async fn query<Q: GraphQLQuery>(&self, variables: Q::Variables) -> Result<Q::ResponseData> {
        let response = self
            .http
            .post(&self.graphql_url)
            .json(&Q::build_query(variables))
            .send()
            .await
            .with_context(|| format!("Failed to reach the GitHub API at {}", self.graphql_url))?;
        let response: GraphQlResponse<Q::ResponseData> =
            check(response, "/graphql").await?.json().await?;
        let errors = response.errors.unwrap_or_default();
        let tolerated = response.data.is_some()
            && errors
                .iter()
                .all(|error| error.kind.as_deref() == Some("NOT_FOUND"));
        if let Some(error) = errors.into_iter().next().filter(|_| !tolerated) {
            bail!("GitHub GraphQL error: {}", sentence(&error.message));
        }
        response
            .data
            .ok_or_else(|| anyhow!("The GitHub GraphQL response has no data."))
    }
}

#[derive(Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
struct ErrorMessage {
    message: String,
}

async fn check(response: reqwest::Response, path: &str) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let message = response
        .json::<ErrorMessage>()
        .await
        .map(|error| format!(": {}", error.message))
        .unwrap_or_default();
    bail!(
        "GitHub API returned {status} for {path}{}",
        sentence(&message)
    )
}

/// Ends a message with a period.
pub fn sentence(message: &str) -> String {
    let message = message.trim_end();
    if message.is_empty() || message.ends_with(['.', '!', '?']) {
        message.to_string()
    } else {
        format!("{message}.")
    }
}

/// An error and its causes as one sentence, such as `Failed to reach the GitHub API at https://api.github.com: connection refused.`.
pub fn error_message(error: &anyhow::Error) -> String {
    let messages: Vec<String> = error
        .chain()
        .map(|cause| cause.to_string().trim_end_matches('.').to_string())
        .collect();
    sentence(&messages.join(": "))
}

/// `https://api.github.com/graphql`, or `https://host/api/graphql` for a GitHub Enterprise Server `https://host/api/v3`.
pub fn graphql_url(api_url: &str) -> String {
    let api_url = api_url.trim_end_matches('/');
    match api_url.strip_suffix("/api/v3") {
        Some(base) => format!("{base}/api/graphql"),
        None => format!("{api_url}/graphql"),
    }
}

/// The host `gh` knows the API by: `github.com` for `https://api.github.com`, otherwise the API's host.
pub fn gh_host(api_url: &str) -> String {
    match Url::parse(api_url)
        .ok()
        .and_then(|url| url.host_str().map(|host| (host.to_string(), url.port())))
    {
        Some((host, _)) if host == "api.github.com" => "github.com".to_string(),
        Some((host, Some(port))) => format!("{host}:{port}"),
        Some((host, None)) => host,
        None => "github.com".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_urls() {
        assert_eq!(
            graphql_url("https://api.github.com"),
            "https://api.github.com/graphql"
        );
        assert_eq!(
            graphql_url("https://api.github.com/"),
            "https://api.github.com/graphql"
        );
        assert_eq!(
            graphql_url("https://ghe.example.com/api/v3"),
            "https://ghe.example.com/api/graphql"
        );
        assert_eq!(
            graphql_url("http://127.0.0.1:8080"),
            "http://127.0.0.1:8080/graphql"
        );
    }

    #[test]
    fn gh_hosts() {
        assert_eq!(gh_host("https://api.github.com"), "github.com");
        assert_eq!(gh_host("https://ghe.example.com/api/v3"), "ghe.example.com");
        assert_eq!(gh_host("http://127.0.0.1:8080"), "127.0.0.1:8080");
        assert_eq!(gh_host("not a url"), "github.com");
    }

    #[test]
    fn ends_messages_with_periods() {
        assert_eq!(sentence("Bad credentials"), "Bad credentials.");
        assert_eq!(sentence("Done."), "Done.");
        assert_eq!(sentence("Really?"), "Really?");
        assert_eq!(sentence(""), "");
    }

    #[test]
    fn joins_error_causes() {
        let error =
            anyhow!("connection refused.").context("Failed to reach the GitHub API at http://x");
        assert_eq!(
            error_message(&error),
            "Failed to reach the GitHub API at http://x: connection refused."
        );
    }
}
