extern crate core;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use fs_extra::dir::CopyOptions;
use itertools::Itertools;
use octocrab::models;
use octocrab::params::{Direction, issues, State};
use octocrab::params::issues::Sort;
use regex::{RegexBuilder};
use semver::Version;
use tap::{Tap};

const NUM_VERSIONS: usize = 5;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_dir_all("hugo/rust-changelogs/content");
    let _ = std::fs::remove_dir_all("hugo/rust-changelogs/public");
    let mut options = CopyOptions::new();
    options.copy_inside = true;
    fs_extra::dir::copy("hugo/rust-changelogs/template", "hugo/rust-changelogs/content", &options).expect("copy hugo dir");

    let body = reqwest::get("https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md").await?.error_for_status()?.text().await?;

    let split_re = RegexBuilder::new("^Version\\s+").multi_line(true).build().unwrap();
    let changelogs: HashMap<_, _> = split_re.split(&body).filter_map(|s| {
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
    }).collect();

    let changelogs_vec: Vec<_> = changelogs.iter().sorted_by_key(|(v, _)| *v).collect();

    let octocrab = octocrab::instance();

    for (idx, (version, (changelog, release_date))) in changelogs_vec.iter().enumerate()
    {
        let mut trimmed = changelog.trim().to_string();
        if trimmed.starts_with("-") {
            trimmed = format!("Changes\n-------\n{}", trimmed);
        }

        let content = format!("---\nweight: {}\n---\n\n{} ({})\n========\n\n{}",
                              1000000 - idx,
                              version, release_date,
                              trimmed);

        std::fs::write(
            format!("hugo/rust-changelogs/content/docs/released/{}.md", version),
            content,
        ).unwrap();
    }

    let mut milestones = HashMap::new();
    let released_versions = changelogs.keys().cloned().collect();

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
                .and_then(|m| semver::Version::parse(&m.title).ok()) {
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

    let issues_versions: HashSet<_> = milestones.keys().cloned().collect();
    let unreleased_versions: HashSet<_> = issues_versions.difference(&released_versions).collect();

    let unreleased_version_to_milestone: Vec<_> = milestones
        .into_iter()
        .filter(|(v, _m)| unreleased_versions.contains(v))
        .map(|(v, m)| (v, m.unwrap().number))
        .sorted_by_key(|(v, _)| v.clone())
        .collect();

    for (idx, (unreleased_version, milestone_id)) in unreleased_version_to_milestone.iter().enumerate() {
        let mut changelog = format!(
            "---\nweight: {}\n---\n\n{} (Unreleased)\n=========\n",
            1000000 - idx,
            unreleased_version,
        );
        let mut issues_page = octocrab
            .issues("rust-lang", "rust")
            .list()
            .milestone(issues::Filter::Matches((*milestone_id).try_into().expect("overflow in milestone_id")))
            .labels(&vec![String::from("relnotes")])
            .per_page(255)
            .sort(Sort::Created)
            .direction(Direction::Ascending)
            .state(State::Closed)
            .send()
            .await?;

        loop {
            for issue in &issues_page {
                changelog.push_str("- [");
                changelog.push_str(issue.title.as_str());
                changelog.push_str("](");
                changelog.push_str(issue.html_url.as_str());
                changelog.push_str(")\n");
            }
            issues_page = match octocrab
                .get_page::<models::issues::Issue>(&issues_page.next)
                .await?
            {
                Some(next_page) => next_page,
                None => break,
            };
        }

        std::fs::write(
            format!("hugo/rust-changelogs/content/docs/unreleased/{}.md", unreleased_version),
            changelog,
        ).unwrap();
    }

    let stable_version = changelogs_vec.last().unwrap().0;
    let beta_version = stable_version.clone().tap_mut(|v| {
        v.minor += 1;
        v.patch = 0;
    });
    let nightly_version = stable_version.clone().tap_mut(|v| {
        v.minor += 2;
        v.patch = 0;
    });

    let mut index = format!("---
title: Rust Versions
type: docs
---

Rust Versions
=============

- Stable: [{stable_version}](/docs/released/{stable_version})
");

    if unreleased_versions.contains(&beta_version) {
        index.push_str(&format!("- Beta: [{beta_version}](/docs/unreleased/{beta_version})\n"));
    };
    if unreleased_versions.contains(&nightly_version) {
        index.push_str(&format!("- Nightly: [{nightly_version}](/docs/unreleased/{nightly_version})\n"));
    };

    std::fs::write(
        "hugo/rust-changelogs/content/_index.md",
        index,
    ).unwrap();

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
