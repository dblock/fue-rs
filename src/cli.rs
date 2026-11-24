//! Command-line arguments and the top-level search.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use futures::future::join_all;

use crate::auth;
use crate::commits::SignOffs;
use crate::github::{self, GitHub, error_message};
use crate::model::Person;
use crate::output::{self, Found};
use crate::repository;
use crate::target::Target;
use crate::user;

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
        value_parser = clap::value_parser!(u32).range(1..),
        help = "Commits to look through in each repository for a user (default 1), or for each contributor of a repository (default 100)."
    )]
    pub depth: Option<u32>,

    #[arg(
        long,
        value_name = "REPOSITORIES",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(1..),
        help = "Repositories to look through for a user, most recently pushed first."
    )]
    pub breadth: u32,

    #[arg(
        long,
        value_name = "CONTRIBUTORS",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(1..),
        help = "Contributors to show for a repository, most commits first."
    )]
    pub limit: u32,

    #[arg(
        long,
        value_enum,
        value_name = "SIGN_OFFS",
        default_value_t = SignOffs::Own,
        help = "Signed-off-by addresses to count as a person's: own, those of the same person, listing others' with --verbose and in JSON; all; or none."
    )]
    pub sign_offs: SignOffs,

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
    let results = join_all(
        args.targets
            .iter()
            .map(|target| search(&github, target, &args)),
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

async fn search(github: &GitHub, target: &Target, args: &Args) -> Result<Vec<Person>> {
    let depth = args.depth.map(|depth| depth as usize);
    match target {
        Target::User(login) => {
            let limits = user::Limits {
                depth: depth.unwrap_or(1),
                breadth: args.breadth as usize,
                sign_offs: args.sign_offs,
            };
            Ok(vec![user::search(github, login, limits).await?])
        }
        Target::Repository { owner, name } => {
            let limits = repository::Limits {
                depth: depth.unwrap_or(100),
                limit: args.limit as usize,
                sign_offs: args.sign_offs,
            };
            repository::search(github, owner, name, limits).await
        }
    }
}
