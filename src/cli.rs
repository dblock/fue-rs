//! Command-line arguments and the top-level search.

use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::Parser;
use futures::future::join_all;

use crate::auth;
use crate::github::{self, GitHub, error_message};
use crate::model::Person;
use crate::output::{self, Found};
use crate::target::Target;
use crate::user::{self, Limits};

#[derive(Debug, Parser)]
#[command(
    name = "fue",
    version,
    about = "Find e-mail addresses of GitHub users and repository contributors."
)]
pub struct Args {
    #[arg(
        required = true,
        value_name = "USER|OWNER/REPO",
        help = "GitHub users, such as `dblock`, or repositories, such as `dblock/fue`."
    )]
    pub targets: Vec<Target>,

    #[arg(
        long,
        value_name = "TOKEN",
        help = "GitHub token. Defaults to GH_TOKEN, GITHUB_TOKEN, or `gh auth token`."
    )]
    pub token: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        default_value = github::DEFAULT_API_URL,
        help = "GitHub API URL, such as `https://ghe.example.com/api/v3` for GitHub Enterprise Server."
    )]
    pub api_url: String,

    #[arg(
        long,
        value_name = "COMMITS",
        default_value_t = 1,
        value_parser = clap::value_parser!(u32).range(1..),
        help = "Commits by the user to look through in each repository."
    )]
    pub depth: u32,

    #[arg(
        long,
        value_name = "REPOSITORIES",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(1..),
        help = "Repositories to look through, most recently pushed first."
    )]
    pub breadth: u32,

    #[arg(long, help = "Print JSON.")]
    pub json: bool,

    #[arg(short, long, help = "Show every name and the sources of each address.")]
    pub verbose: bool,
}

pub async fn run(args: Args) -> Result<ExitCode> {
    let token = auth::find_token(
        args.token.as_deref(),
        |name| std::env::var(name).ok(),
        auth::gh_auth_token,
        &github::gh_host(&args.api_url),
    )?;
    if args.verbose {
        eprintln!("Using a GitHub token from {}.", token.source);
    }
    let github = GitHub::new(&args.api_url, &token.value)?;
    let limits = Limits {
        depth: args.depth as usize,
        breadth: args.breadth as usize,
    };
    let results = join_all(
        args.targets
            .iter()
            .map(|target| search(&github, target, limits)),
    )
    .await;

    let mut failed = false;
    let mut found = Vec::new();
    for (target, result) in args.targets.iter().zip(results) {
        match result {
            Ok(people) => found.push((target.to_string(), people)),
            Err(error) => {
                eprintln!("error: {target}: {}", error_message(&error));
                failed = true;
            }
        }
    }

    if args.json {
        let found: Vec<Found<'_>> = found
            .iter()
            .map(|(target, people)| Found {
                target: target.clone(),
                people,
            })
            .collect();
        println!("{}", output::json(&found));
    } else {
        let people: Vec<Person> = found.into_iter().flat_map(|(_, people)| people).collect();
        print!("{}", output::text(&people, args.verbose));
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

async fn search(github: &GitHub, target: &Target, limits: Limits) -> Result<Vec<Person>> {
    match target {
        Target::User(login) => Ok(vec![user::search(github, login, limits).await?]),
        Target::Repository { .. } => bail!("Searching repositories is not supported yet."),
    }
}
