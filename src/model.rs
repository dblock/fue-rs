//! People and the e-mail addresses found for them.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// Where an address was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// The public e-mail on a GitHub profile.
    Profile,
    /// Commits in recent public activity.
    Activity,
    /// Commit history.
    History,
    /// `Signed-off-by:` lines in commit messages.
    SignedOffBy,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Profile => "profile",
            Source::Activity => "activity",
            Source::History => "history",
            Source::SignedOffBy => "signed-off-by",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Address {
    pub email: String,
    /// Names used with this address, most used first.
    pub names: Vec<String>,
    /// Distinct commits that used this address.
    pub commits: usize,
    /// Date of the most recent commit, as `YYYY-MM-DD`.
    pub last_used: Option<String>,
    /// Whether this is a `users.noreply` address, which does not receive e-mail.
    pub noreply: bool,
    pub sources: Vec<Source>,
}

/// A GitHub user, or a commit author without a GitHub account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Person {
    pub login: Option<String>,
    pub name: Option<String>,
    pub addresses: Vec<Address>,
    /// Addresses in `Signed-off-by:` lines of this person's commits that belong to other people, such as reviewers.
    pub signed_off_by_others: Vec<Address>,
}

#[derive(Default)]
struct Entry {
    email: String,
    names: BTreeMap<String, usize>,
    commits: BTreeSet<String>,
    last_used: Option<String>,
    sources: BTreeSet<Source>,
}

/// Collects addresses from every source, merging them case-insensitively.
#[derive(Default)]
pub struct AddressBook {
    entries: BTreeMap<String, Entry>,
}

impl AddressBook {
    /// Adds an address. `commit` is a commit SHA and its ISO 8601 date, so a commit seen by two sources is counted once.
    pub fn add(
        &mut self,
        email: &str,
        name: Option<&str>,
        source: Source,
        commit: Option<(&str, &str)>,
    ) {
        let email = email.trim();
        if email.is_empty() || !email.contains('@') {
            return;
        }
        let entry = self
            .entries
            .entry(email.to_lowercase())
            .or_insert_with(|| Entry {
                email: email.to_string(),
                ..Entry::default()
            });
        if let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) {
            *entry.names.entry(name.to_string()).or_default() += 1;
        }
        entry.sources.insert(source);
        if let Some((sha, date)) = commit {
            entry.commits.insert(sha.to_string());
            let date: String = date.chars().take(10).collect();
            if entry.last_used.as_ref().is_none_or(|last| *last < date) {
                entry.last_used = Some(date);
            }
        }
    }

    /// Whether the book has an address, compared case-insensitively.
    pub fn contains(&self, email: &str) -> bool {
        self.entries.contains_key(&email.trim().to_lowercase())
    }

    /// Real addresses first, then most recently used, then most used.
    pub fn into_addresses(self) -> Vec<Address> {
        let mut addresses: Vec<Address> = self
            .entries
            .into_values()
            .map(|entry| {
                let mut names: Vec<(String, usize)> = entry.names.into_iter().collect();
                names.sort_by(|(a, a_count), (b, b_count)| b_count.cmp(a_count).then(a.cmp(b)));
                Address {
                    noreply: is_noreply(&entry.email),
                    email: entry.email,
                    names: names.into_iter().map(|(name, _)| name).collect(),
                    commits: entry.commits.len(),
                    last_used: entry.last_used,
                    sources: entry.sources.into_iter().collect(),
                }
            })
            .collect();
        addresses.sort_by(|a, b| {
            a.noreply
                .cmp(&b.noreply)
                .then_with(|| b.last_used.cmp(&a.last_used))
                .then_with(|| b.commits.cmp(&a.commits))
                .then_with(|| a.email.to_lowercase().cmp(&b.email.to_lowercase()))
        });
        addresses
    }
}

/// `users.noreply.github.com` addresses, or `users.noreply.<host>` on GitHub Enterprise Server.
pub fn is_noreply(email: &str) -> bool {
    email
        .rsplit_once('@')
        .is_some_and(|(_, domain)| domain.to_lowercase().starts_with("users.noreply."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_addresses_case_insensitively() {
        let mut book = AddressBook::default();
        book.add(
            "DB@Example.com",
            Some("DB"),
            Source::History,
            Some(("a", "2019-04-02T10:00:00Z")),
        );
        book.add(
            "db@example.com",
            Some("Dee"),
            Source::Activity,
            Some(("b", "2020-01-01T00:00:00Z")),
        );
        book.add(
            "db@example.com",
            Some("Dee"),
            Source::History,
            Some(("c", "2018-01-01T00:00:00Z")),
        );
        let addresses = book.into_addresses();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].email, "DB@Example.com");
        assert_eq!(addresses[0].names, vec!["Dee", "DB"]);
        assert_eq!(addresses[0].commits, 3);
        assert_eq!(addresses[0].last_used.as_deref(), Some("2020-01-01"));
        assert_eq!(
            addresses[0].sources,
            vec![Source::Activity, Source::History]
        );
    }

    #[test]
    fn finds_addresses_case_insensitively() {
        let mut book = AddressBook::default();
        book.add("DB@Example.com", None, Source::Profile, None);
        assert!(book.contains(" db@example.com "));
        assert!(!book.contains("other@example.com"));
    }

    #[test]
    fn counts_a_commit_once() {
        let mut book = AddressBook::default();
        book.add(
            "a@example.com",
            None,
            Source::Activity,
            Some(("sha", "2020-01-01")),
        );
        book.add(
            "a@example.com",
            None,
            Source::History,
            Some(("sha", "2020-01-01")),
        );
        assert_eq!(book.into_addresses()[0].commits, 1);
    }

    #[test]
    fn sorts_real_addresses_first_then_most_recent() {
        let mut book = AddressBook::default();
        book.add(
            "1+a@users.noreply.github.com",
            None,
            Source::History,
            Some(("n", "2026-01-01")),
        );
        book.add(
            "old@example.com",
            None,
            Source::History,
            Some(("o", "2019-01-01")),
        );
        book.add(
            "new@example.com",
            None,
            Source::History,
            Some(("p", "2025-01-01")),
        );
        book.add("profile@example.com", None, Source::Profile, None);
        let emails: Vec<String> = book.into_addresses().into_iter().map(|a| a.email).collect();
        assert_eq!(
            emails,
            [
                "new@example.com",
                "old@example.com",
                "profile@example.com",
                "1+a@users.noreply.github.com"
            ]
        );
    }

    #[test]
    fn breaks_ties_by_commits_then_address() {
        let mut book = AddressBook::default();
        book.add(
            "b@example.com",
            None,
            Source::History,
            Some(("1", "2020-01-01")),
        );
        book.add(
            "a@example.com",
            None,
            Source::History,
            Some(("2", "2020-01-01")),
        );
        book.add(
            "c@example.com",
            None,
            Source::History,
            Some(("3", "2020-01-01")),
        );
        book.add(
            "c@example.com",
            None,
            Source::History,
            Some(("4", "2019-01-01")),
        );
        let emails: Vec<String> = book.into_addresses().into_iter().map(|a| a.email).collect();
        assert_eq!(emails, ["c@example.com", "a@example.com", "b@example.com"]);
    }

    #[test]
    fn ignores_invalid_addresses() {
        let mut book = AddressBook::default();
        book.add("", None, Source::Profile, None);
        book.add("not-an-address", None, Source::Profile, None);
        book.add("a@example.com", Some(" "), Source::Profile, None);
        let addresses = book.into_addresses();
        assert_eq!(addresses.len(), 1);
        assert!(addresses[0].names.is_empty());
    }

    #[test]
    fn detects_noreply_addresses() {
        assert!(is_noreply("1+octocat@users.noreply.github.com"));
        assert!(is_noreply("octocat@USERS.NOREPLY.GITHUB.COM"));
        assert!(is_noreply("octocat@users.noreply.ghe.example.com"));
        assert!(!is_noreply("octocat@example.com"));
        assert!(!is_noreply("noreply"));
    }

    #[test]
    fn labels_sources() {
        let labels: Vec<&str> = [
            Source::Profile,
            Source::Activity,
            Source::History,
            Source::SignedOffBy,
        ]
        .into_iter()
        .map(Source::label)
        .collect();
        assert_eq!(labels, ["profile", "activity", "history", "signed-off-by"]);
    }
}
