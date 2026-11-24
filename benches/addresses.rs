//! Benchmarks merging, sorting, and formatting addresses, and parsing commits, which run for every commit fue reads.
//!
//! Run with `cargo bench --bench addresses`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use fue::commits::{self, Commit, SignOffs};
use fue::model::{AddressBook, Person, Source};
use fue::output;

/// Commits by one person with `addresses` distinct addresses, each signed off.
fn commits(count: usize, addresses: usize) -> Vec<Commit> {
    (0..count)
        .map(|index| Commit {
            oid: format!("{index:040x}"),
            committed_date: format!("2026-{:02}-{:02}T00:00:00Z", 1 + index % 12, 1 + index % 28),
            message: format!(
                "Change {index}.\n\nSigned-off-by: Octo Cat <octocat{}@example.com>\nSigned-off-by: Mona Lisa <mona@example.com>",
                index % addresses
            ),
            author_name: Some("Octo Cat".to_string()),
            author_email: Some(format!("octocat{}@example.com", index % addresses)),
        })
        .collect()
}

fn addresses(criterion: &mut Criterion) {
    let commits = commits(10_000, 50);
    let all: Vec<(Source, &Commit)> = commits
        .iter()
        .map(|commit| (Source::History, commit))
        .collect();
    criterion.bench_function("add 10000 commits", |bencher| {
        bencher.iter(|| {
            let mut addresses = AddressBook::default();
            let mut others = AddressBook::default();
            commits::add_commits(&mut addresses, &mut others, black_box(&all), SignOffs::Own);
            (addresses.into_addresses(), others.into_addresses())
        });
    });

    let mut book = AddressBook::default();
    let mut others = AddressBook::default();
    commits::add_commits(&mut book, &mut others, &all, SignOffs::Own);
    let people = [Person {
        login: Some("octocat".to_string()),
        name: Some("Octo Cat".to_string()),
        addresses: book.into_addresses(),
        signed_off_by_others: others.into_addresses(),
    }];
    criterion.bench_function("format 50 addresses", |bencher| {
        bencher.iter(|| output::text(black_box(&people), true));
    });

    let message = &commits[0].message;
    criterion.bench_function("parse sign-offs", |bencher| {
        bencher.iter(|| commits::signed_off_by(black_box(message)));
    });
    criterion.bench_function("match names", |bencher| {
        bencher.iter(|| {
            commits::same_name(
                black_box("Daniel Doubrovkine"),
                black_box("Daniel (dB.) Doubrovkine"),
            )
        });
    });
}

criterion_group!(benches, addresses);
criterion_main!(benches);
