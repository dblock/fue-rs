//! Find e-mail addresses of GitHub users and repository contributors.

use std::process::ExitCode;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "fue",
    version,
    about = "Find e-mail addresses of GitHub users and repository contributors."
)]
struct Args {
    #[arg(
        required = true,
        value_name = "USER|OWNER/REPO",
        help = "GitHub users, such as `dblock`, or repositories, such as `dblock/fue`."
    )]
    targets: Vec<String>,
}

fn main() -> ExitCode {
    let _args = Args::parse();
    eprintln!("error: Searching is not implemented yet.");
    ExitCode::FAILURE
}
