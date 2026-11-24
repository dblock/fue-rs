//! Runs the `fue` binary.

use assert_cmd::Command;
use predicates::prelude::*;

fn fue() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("fue")
}

#[test]
fn prints_the_version() {
    fue()
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("fue {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn prints_help() {
    fue()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Find e-mail addresses of GitHub users and repository contributors.",
        ))
        .stdout(predicate::str::contains("<USER|OWNER/REPO>..."))
        .stdout(predicate::str::contains(
            "repositories, such as `dblock/fue`.",
        ));
}

#[test]
fn requires_a_target() {
    fue()
        .assert()
        .code(2)
        .stderr(predicate::str::contains("<USER|OWNER/REPO>"));
}

#[test]
fn does_not_search_yet() {
    fue()
        .arg("octocat")
        .assert()
        .failure()
        .stdout("")
        .stderr("error: Searching is not implemented yet.\n");
}
