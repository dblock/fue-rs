//! Formats the people found as readable text or JSON.

use serde::Serialize;

use crate::model::{Address, Person, Source};

/// The people found for one target, in `--json` output.
#[derive(Debug, Serialize)]
pub struct Found<'a> {
    pub target: String,
    pub people: &'a [Person],
}

pub fn json(found: &[Found<'_>]) -> String {
    serde_json::to_string_pretty(found).expect("people serialize to JSON")
}

/// One block per person: a heading, then one aligned line per address.
pub fn text(people: &[Person], verbose: bool) -> String {
    people
        .iter()
        .map(|person| person_text(person, verbose))
        .collect::<Vec<_>>()
        .join("\n")
}

fn person_text(person: &Person, verbose: bool) -> String {
    let heading = match (&person.login, &person.name) {
        (Some(login), _) => format!("@{login}"),
        (None, Some(name)) => format!("{name} (no GitHub account)"),
        (None, None) => "(unknown)".to_string(),
    };
    let mut lines = vec![heading];
    if person.addresses.is_empty() {
        lines.push("  (no e-mail found)".to_string());
    }
    lines.extend(table(&person.addresses, verbose, "  "));
    if verbose && !person.signed_off_by_others.is_empty() {
        lines.push("  Signed off by others:".to_string());
        lines.extend(table(&person.signed_off_by_others, verbose, "    "));
    }
    lines.join("\n") + "\n"
}

/// One aligned line per address, each starting with `indent`.
fn table(addresses: &[Address], verbose: bool, indent: &str) -> Vec<String> {
    let rows: Vec<[String; 4]> = addresses
        .iter()
        .map(|address| row(address, verbose))
        .collect();
    let widths: Vec<usize> = (0..3)
        .map(|column| {
            rows.iter()
                .map(|row| row[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|row| {
            let mut line = String::new();
            for (column, value) in row.iter().enumerate() {
                // Columns that are empty in every row, such as names nobody used, are left out.
                let width = widths.get(column).copied();
                if width == Some(0) || (width.is_none() && value.is_empty()) {
                    continue;
                }
                line.push_str(if line.is_empty() { indent } else { "  " });
                line.push_str(value);
                if let Some(width) = width {
                    line.push_str(&" ".repeat(width - value.chars().count()));
                }
            }
            line.trim_end().to_string()
        })
        .collect()
}

fn row(address: &Address, verbose: bool) -> [String; 4] {
    let name = if verbose {
        address.names.join(", ")
    } else {
        address.names.first().cloned().unwrap_or_default()
    };
    let usage = match address.commits {
        0 if address.sources.contains(&Source::Profile) => "public profile".to_string(),
        // A contributor without a GitHub account whose commits were not found.
        0 => String::new(),
        count => {
            let commits = if count == 1 { "commit" } else { "commits" };
            match &address.last_used {
                Some(date) => format!("{count} {commits}, last {date}"),
                None => format!("{count} {commits}"),
            }
        }
    };
    let mut tags = Vec::new();
    if address.noreply {
        tags.push("noreply".to_string());
    }
    if verbose {
        let sources: Vec<&str> = address
            .sources
            .iter()
            .map(|source| source.label())
            .collect();
        tags.push(format!("via {}", sources.join(", ")));
    }
    [address.email.clone(), name, usage, tags.join("  ")]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(
        email: &str,
        names: &[&str],
        commits: usize,
        last_used: Option<&str>,
        sources: &[Source],
    ) -> Address {
        Address {
            email: email.to_string(),
            names: names.iter().map(|name| name.to_string()).collect(),
            commits,
            last_used: last_used.map(str::to_string),
            noreply: crate::model::is_noreply(email),
            sources: sources.to_vec(),
        }
    }

    fn octocat() -> Person {
        Person {
            login: Some("octocat".to_string()),
            name: None,
            addresses: vec![
                address(
                    "octocat@example.com",
                    &["The Octocat", "octo"],
                    412,
                    Some("2026-11-23"),
                    &[Source::History],
                ),
                address("o@example.com", &[], 0, None, &[Source::Profile]),
                address(
                    "1+octocat@users.noreply.github.com",
                    &["octocat"],
                    1,
                    Some("2024-01-10"),
                    &[Source::Activity, Source::History],
                ),
            ],
            signed_off_by_others: vec![address(
                "mona@example.com",
                &["Mona Lisa"],
                1,
                Some("2025-01-02"),
                &[Source::SignedOffBy],
            )],
        }
    }

    #[test]
    fn aligns_addresses() {
        assert_eq!(
            text(&[octocat()], false),
            "@octocat\n\
             \x20 octocat@example.com                 The Octocat  412 commits, last 2026-11-23\n\
             \x20 o@example.com                                    public profile\n\
             \x20 1+octocat@users.noreply.github.com  octocat      1 commit, last 2024-01-10     noreply\n"
        );
    }

    #[test]
    fn shows_all_names_and_sources_when_verbose() {
        let output = text(&[octocat()], true);
        assert!(output.contains("The Octocat, octo"), "{output}");
        assert!(
            output.contains("noreply  via activity, history"),
            "{output}"
        );
        assert!(output.contains("via profile"), "{output}");
    }

    #[test]
    fn lists_others_sign_offs_only_when_verbose() {
        assert!(!text(&[octocat()], false).contains("Signed off by others"));
        let output = text(&[octocat()], true);
        assert!(
            output.ends_with(
                "  Signed off by others:\n    mona@example.com  Mona Lisa  1 commit, last 2025-01-02  via signed-off-by\n"
            ),
            "{output}"
        );
    }

    #[test]
    fn separates_people_and_shows_missing_addresses() {
        let people = [
            Person {
                login: Some("nobody".to_string()),
                name: None,
                addresses: vec![],
                signed_off_by_others: vec![],
            },
            Person {
                login: None,
                name: Some("Some Person".to_string()),
                addresses: vec![address("p@example.com", &[], 2, None, &[Source::History])],
                signed_off_by_others: vec![],
            },
            Person {
                login: None,
                name: None,
                addresses: vec![],
                signed_off_by_others: vec![],
            },
        ];
        assert_eq!(
            text(&people, false),
            "@nobody\n  (no e-mail found)\n\nSome Person (no GitHub account)\n  p@example.com  2 commits\n\n(unknown)\n  (no e-mail found)\n"
        );
    }

    #[test]
    fn formats_json() {
        let people = [octocat()];
        let output = json(&[Found {
            target: "octocat".to_string(),
            people: &people,
        }]);
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value[0]["target"], "octocat");
        assert_eq!(value[0]["people"][0]["login"], "octocat");
        assert_eq!(value[0]["people"][0]["addresses"][0]["commits"], 412);
        assert_eq!(value[0]["people"][0]["addresses"][2]["noreply"], true);
        assert_eq!(
            value[0]["people"][0]["addresses"][2]["sources"][0],
            "activity"
        );
    }
}
