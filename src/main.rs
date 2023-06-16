extern crate core;

use chrono::{Duration, NaiveDate, Utc};
use fs_extra::dir::CopyOptions;
use itertools::Itertools;
use octocrab::models::issues::Issue;
use octocrab::params::issues::Sort;
use octocrab::params::{issues, Direction, State};
use octocrab::{models, Octocrab};
use regex::RegexBuilder;
use semver::Version;
use std::collections::{HashMap, HashSet};
use std::{fs, io};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tap::Tap;

const NUM_VERSIONS: usize = 5;

struct ReleaseDate {
    release_date: NaiveDate,
    branch_date: NaiveDate,
}

// https://forge.rust-lang.org/js/index.js
fn calculate_release_date(incr: u32) -> ReleaseDate {
    let epoch_date: NaiveDate = NaiveDate::from_ymd(2015, 12, 10);
    let new_releases = ((Utc::now().date_naive() - epoch_date).num_weeks() as f64 / 6.0).floor() as u32;
    let release_date = epoch_date + Duration::weeks(((new_releases + incr) * 6).into());
    let branch_date = epoch_date + Duration::weeks(((new_releases + incr - 1) * 6).into()) - Duration::days(6);

    ReleaseDate {
        release_date,
        branch_date,
    }
}


fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        fs::remove_file(entry?.path())?;
    }
    Ok(())
}

// Determines the order of releases in the side bar
const fn determine_weight(Version { major, minor, patch, .. }: &Version) -> u32 {
    u32::MAX
        - ( ( *major as u32 ) << 24 )
        - ( ( *minor as u32 ) << 8 )
        - *patch as u32
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fs::remove_dir_all("hugo/rust-changelogs/content");
    let _ = remove_dir_contents("hugo/rust-changelogs/public");
    let mut options = CopyOptions::new();
    options.copy_inside = true;
    fs_extra::dir::copy(
        "hugo/rust-changelogs/template",
        "hugo/rust-changelogs/content",
        &options,
    )
        .expect("copy hugo dir");

    let body = reqwest::get("https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md")
        .await?
        .error_for_status()?
        .text()
        .await?;

    let split_re = RegexBuilder::new("^Version\\s+")
        .multi_line(true)
        .build()
        .unwrap();
    let changelogs: HashMap<_, _> = split_re
        .split(&body)
        .filter_map(|s| {
            if let Some(ws_idx) = s.find(|c: char| c.is_whitespace()) {
                let rest = &s[ws_idx..];
                let version = &s[0..ws_idx];
                // 2022-05-19
                let time: chrono::NaiveDate = s[ws_idx + 1..].trim_start()[1..11].parse().unwrap();
                if let Ok(version) = semver::Version::parse(version) {
                    if version > Version::from_str("1.0.0").unwrap() {
                        let changelog = rest.lines().skip(2).collect::<Vec<_>>().join("\n");
                        Some((version, (changelog, time)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let changelogs_vec: Vec<_> = changelogs.iter().sorted_by_key(|(v, _)| *v).collect();

    let octocrab = Arc::new(Octocrab::builder().build().unwrap());

    for (version, (changelog, release_date)) in changelogs_vec.iter() {
        let mut trimmed = changelog.trim().to_string();
        if trimmed.starts_with("-") {
            trimmed = format!("Changes\n-------\n{}", trimmed);
        }

        let content = format!(
            "---\nweight: {}\n---\n\n{} ({})\n========\n\n{}",
            determine_weight(version),
            version,
            release_date.format("%-d %B, %C%y"),
            trimmed
        );

        if *release_date <= Utc::now().naive_utc().date() {
            std::fs::write(
                format!("hugo/rust-changelogs/content/docs/{}.md", version),
                content,
            )
                .unwrap();
        } else {
            std::fs::write(
                format!(
                    "hugo/rust-changelogs/content/docs/{}.md",
                    version
                ),
                content,
            )
                .unwrap();
        }
    }

    let mut milestones = HashMap::new();
    let released_versions = changelogs
        .iter()
        .filter(|(_, (_, date))| *date <= Utc::now().naive_utc().date())
        .map(|(k, _)| k.clone())
        .collect();

    let mut issues_page = octocrab
        .issues("rust-lang", "rust")
        .list()
        .labels(&vec![String::from("relnotes")])
        .per_page(255)
        .sort(Sort::Created)
        .direction(Direction::Descending)
        .state(State::Closed)
        .send()
        .await?;

    'issues_pages: loop {
        for issue in &issues_page {
            if let Some(version) = issue
                .milestone
                .as_ref()
                .and_then(|m| semver::Version::parse(&m.title).ok())
            {
                milestones.entry(version).or_insert(issue.milestone.clone());

                if milestones.len() > NUM_VERSIONS {
                    break 'issues_pages;
                };
            }
        }
        issues_page = match octocrab
            .get_page::<models::issues::Issue>(&issues_page.next)
            .await?
        {
            Some(next_page) => next_page,
            None => break,
        };
    }

    let mut stabilization_prs = HashMap::new();

    for search_term in [
        "stabilise",
        "Stabilise",
        "Stabilize",
        "stabilize",
        "stabilisation",
        "Stabilisation",
        "Stabilization",
        "stabilization"
    ] {
        println!("search for {} PRs", search_term);

        let mut prs_page = octocrab::instance()
            .search()
            .issues_and_pull_requests(&format!("is:pr is:open repo:rust-lang/rust {}", search_term))
            .sort("created_at")
            .order("desc")
            .send()
            .await?;

        loop {
            for pr in &prs_page {
                if pr.title.starts_with(search_term) || pr.title.to_lowercase().starts_with(&format!("partial {search_term}")) {
                    stabilization_prs.insert(pr.id, pr.clone());
                }
            }
            prs_page = match octocrab
                .get_page::<models::issues::Issue>(&prs_page.next)
                .await?
            {
                Some(next_page) => next_page,
                None => break,
            };
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    let issues_versions: HashSet<_> = milestones.keys().cloned().collect();
    let unreleased_versions: HashSet<_> = issues_versions.difference(&released_versions).collect();

    let unreleased_version_to_milestone: Vec<_> = milestones
        .into_iter()
        .filter(|(v, _m)| unreleased_versions.contains(v))
        .map(|(v, m)| (v, m.unwrap().number))
        .sorted_by_key(|(v, _)| v.clone())
        .rev()
        .collect();

    let stable_version = changelogs_vec
        .iter()
        .filter(|(_, (_, release_date))| {
            *release_date <= Utc::now().naive_utc().date()
        })
        .last()
        .unwrap().0;
    let beta_version = stable_version.clone().tap_mut(|v| {
        v.minor += 1;
        v.patch = 0;
    });
    let nightly_version = stable_version.clone().tap_mut(|v| {
        v.minor += 2;
        v.patch = 0;
    });

    for (unreleased_version, milestone_id) in
    unreleased_version_to_milestone.iter().sorted_by_key(|(v, _)| v)
    {
        let release_name = if unreleased_version == &nightly_version {
            "nightly"
        } else if unreleased_version == &beta_version {
            "beta"
        } else if unreleased_version == &nightly_version {
            "nightly"
        } else {
            ""
        };
        let release_date = calculate_release_date((unreleased_version.minor - stable_version.minor) as u32);
        let mut changelog = format!(
            "---
weight: {}

---

{} {}
=========

{{{{< hint warning >}}}}
**Unreleased{}**

- Will be stable on: _{}_
- Will branch from master on: _{}_
{{{{< /hint >}}}}

",
            determine_weight(unreleased_version),
            unreleased_version,
            release_name,
            if Utc::now().naive_utc().date() > release_date.branch_date { ", branched from master" } else { "" },
            release_date.release_date.format("%-d %B, %C%y"),
            release_date.branch_date.format("%-d %B, %C%y"),
        );
        let mut issues_page = octocrab
            .issues("rust-lang", "rust")
            .list()
            .milestone(issues::Filter::Matches(
                (*milestone_id)
                    .try_into()
                    .expect("overflow in milestone_id"),
            ))
            .labels(&vec![String::from("relnotes")])
            .per_page(255)
            .sort(Sort::Created)
            .direction(Direction::Ascending)
            .state(State::Closed)
            .send()
            .await?;

        loop {
            for (issue, days_ago) in (&issues_page).into_iter().filter_map(|issue| {
                if let Some(closed_at) = issue.closed_at {
                    Some((issue, (Utc::now().naive_utc().date() - closed_at.naive_utc().date()).num_days()))
                } else {
                    None
                }
            }).sorted_by_key(|(_, days_ago)| *days_ago) {
                changelog.push_str("- [");
                changelog.push_str(issue.title.as_str());
                changelog.push_str("](");
                changelog.push_str(issue.html_url.as_str());
                changelog.push_str(")");
                let days_ago_text = pluralizer::pluralize("day", days_ago as isize, true);
                changelog.push_str(&format!(" _(merged {} ago)_", days_ago_text));
                changelog.push_str("\n");
            }
            issues_page = match octocrab
                .get_page::<models::issues::Issue>(&issues_page.next)
                .await?
            {
                Some(next_page) => next_page,
                None => break,
            };
        }

        if !changelogs.contains_key(unreleased_version) {
            std::fs::write(
                format!(
                    "hugo/rust-changelogs/content/docs/{}.md",
                    unreleased_version
                ),
                changelog,
            )
                .unwrap();
        }
    }

    let mut index = format!(
        "---
title: Rust Versions
type: docs
---

## Rust Versions

- Stable: [{stable_version}](/docs/{stable_version})
"
    );

    if unreleased_versions.contains(&beta_version) {
        let release_date = calculate_release_date(1);
        let days_left = (release_date.release_date - Utc::now().naive_utc().date()).num_days();
        let days_left_text = pluralizer::pluralize("day", days_left as isize, true);

        index.push_str(&format!(
            "- Beta: [{beta_version}](/docs/{beta_version}) ({}, {} left)\n"
            , release_date.release_date.format("%-d %B, %C%y"),
            days_left_text,
        ));
    };
    if unreleased_versions.contains(&nightly_version) {
        let release_date = calculate_release_date(2);
        let days_left = (release_date.release_date - Utc::now().naive_utc().date()).num_days();
        let days_left_text = pluralizer::pluralize("day", days_left as isize, true);

        index.push_str(&format!(
            "- Nightly: [{nightly_version}](/docs/{nightly_version}) ({}, {} left)\n"
            , release_date.release_date.format("%-d %B, %C%y"),
            days_left_text
        ));
    };

    index.push_str(
        "

## Ongoing Stabilization PRs

",
    );

    for Issue {
        title,
        number,
        html_url,
        created_at,
        labels,
        ..
    } in stabilization_prs
        .into_iter()
        .map(|(_, v)| v)
        .sorted_by_key(|l| l.created_at)
        .rev()
    {
        let days_ago = (Utc::now() - created_at).num_days();
        let days_ago_text = pluralizer::pluralize("day", days_ago as isize, true);
        let mut line = "".to_string();
        let title = title.replace('"', "\\\"");
        line.push_str(&format!("{{{{<details \"{title} ({days_ago_text} old)\">}}}}\n"));
        labels
            .into_iter()
            .for_each(|label| {
                line.push_str("* _");
                line.push_str(&label.name);
                line.push_str("_");
                if let Some(d) = label.description {
                    line.push_str(" - ");
                    line.push_str(&d);
                }
                line.push_str("\n");
            });
        line.push_str(&format!(
            "\n[Open PR #{number}]({html_url})\n\n"
        ));
        line.push_str("{{</details>}}\n");
        index.push_str(&line);
    }

    index.push_str(&format!(
        "

## About releases.rs

- [Github Repo](https://github.com/glebpom/rust-changelogs/)
- Generated at _{}_

", Utc::now().to_rfc2822()));

    std::fs::write("hugo/rust-changelogs/content/_index.md", index).unwrap();

    let res = std::process::Command::new("hugo")
        .arg("--minify")
        .arg("--debug")
        .arg("--theme")
        .arg("hugo-book")
        .current_dir("hugo/rust-changelogs")
        .output()
        .unwrap();
    println!("{}", std::str::from_utf8(&res.stdout).unwrap());
    println!("{}", std::str::from_utf8(&res.stderr).unwrap());

    if !res.status.success() {
        panic!("bad hugo status");
    }

    Ok(())
}
