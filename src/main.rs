//! The `fue` command.

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    match fue::cli::run(fue::cli::Args::parse()).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {}", fue::github::error_message(&error));
            ExitCode::FAILURE
        }
    }
}
