use crate::version_manager::VersionManager;
use chrono::{Duration, NaiveDate, Utc};
use itertools::Itertools;
use octocrab::models::issues::Issue;
use octocrab::models::IssueId;
use semver::Version;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct ChangelogGenerator {
    version_manager: VersionManager,
}

impl ChangelogGenerator {
    pub fn new(version_manager: VersionManager) -> Self {
        Self { version_manager }
    }

    pub fn generate_released_version_content(&self, version: &Version, changelog: &str, release_date: &NaiveDate) -> String {
        let mut trimmed = changelog.trim().to_string();
        if trimmed.starts_with('-') {
            trimmed = format!("Changes\n-------\n{trimmed}");
        }

        let dates = self.version_manager.calculate_release_date(*release_date - Duration::days(1), 1);
        let version_branch_info_str = if version.patch == 0 {
            format!(
                "- Branched from master on: _{branch_date}_",
                branch_date = dates.branch_date.format("%-d %B, %C%y")
            )
        } else {
            "- This is a patch release".to_string()
        };

        format!(
            "---
weight: {weight}

---

{version}
=========

{{{{% hint info %}}}}
- Released on: _{release_date}_
{version_branch_info_str}
{{{{% /hint %}}}}

{trimmed}
",
            weight = self.version_manager.determine_weight(version),
            release_date = release_date.format("%-d %B, %C%y"),
            version_branch_info_str = version_branch_info_str,
        )
    }

    pub fn generate_unreleased_version_content(&self, unreleased_version: &Version, _milestone_id: i64, 
                                          stable_version: &Version, issues: &[Issue]) -> String {
        let release_name = if unreleased_version.minor == stable_version.minor + 2 {
            "nightly"
        } else if unreleased_version.minor == stable_version.minor + 1 {
            "beta"
        } else {
            ""
        };

        let release_date = self.version_manager.calculate_release_date(
            Utc::now().date_naive(),
            (unreleased_version.minor - stable_version.minor) as u32,
        );
        let already_branched = Utc::now().naive_utc().date() > release_date.branch_date;

        let mut changelog = format!(
            "---
weight: {weight}

---

{unreleased_version} {release_name}
=========

{{{{% hint warning %}}}}
**Unreleased{release_sfx}**

- Will be stable on: _{stable_date}_
- {branch_pfx} from master on: _{branch_date}_
{{{{% /hint %}}}}

",
            weight = self.version_manager.determine_weight(unreleased_version),
            release_sfx = if already_branched { ", branched from master" } else { "" },
            stable_date = release_date.release_date.format("%-d %B, %C%y"),
            branch_pfx = if already_branched { "Branched" } else { "Will branch" },
            branch_date = release_date.branch_date.format("%-d %B, %C%y"),
        );

        for (issue, days_ago) in issues.iter()
            .filter_map(|issue| {
                issue.closed_at.map(|closed_at| {
                    (issue, (Utc::now().naive_utc().date() - closed_at.naive_utc().date()).num_days())
                })
            })
            .sorted_by_key(|(_, days_ago)| *days_ago)
        {
            changelog.push_str("- [");
            changelog.push_str(issue.title.as_str());
            changelog.push_str("](");
            changelog.push_str(issue.html_url.as_str());
            changelog.push(')');
            let days_ago_text = pluralizer::pluralize("day", days_ago as isize, true);
            changelog.push_str(&format!(" _(merged {days_ago_text} ago)_"));
            changelog.push('\n');
        }

        changelog
    }

    pub fn generate_index_content(&self, stable_version: &Version, beta_version: &Version, nightly_version: &Version,
                             unreleased_versions: &HashSet<&Version>, stabilization_prs: HashMap<IssueId, Issue>) -> String {
        let mut index = format!(
            "---
title: Rust Versions
type: docs
---

## Rust Versions

- Stable: [{stable_version}](/docs/{stable_version})
"
        );

        if unreleased_versions.contains(beta_version) {
            let release_date = self.version_manager.calculate_release_date(Utc::now().date_naive(), 1);
            let days_left = (release_date.release_date - Utc::now().naive_utc().date()).num_days();
            let days_left_text = pluralizer::pluralize("day", days_left as isize, true);

            index.push_str(&format!(
                "- Beta: [{beta_version}](/docs/{beta_version}) ({}, {days_left_text} left)\n",
                release_date.release_date.format("%-d %B, %C%y"),
            ));
        }

        if unreleased_versions.contains(nightly_version) {
            let release_date = self.version_manager.calculate_release_date(Utc::now().date_naive(), 2);
            let days_left = (release_date.release_date - Utc::now().naive_utc().date()).num_days();
            let days_left_text = pluralizer::pluralize("day", days_left as isize, true);

            index.push_str(&format!(
                "- Nightly: [{nightly_version}](/docs/{nightly_version}) ({}, {days_left_text} left)\n",
                release_date.release_date.format("%-d %B, %C%y")
            ));
        }

        index.push_str("

## Ongoing Stabilization PRs

");

        for Issue {
            title,
            number,
            html_url,
            created_at,
            labels,
            ..
        } in stabilization_prs.into_values().sorted_by_key(|l| l.created_at).rev()
        {
            let days_ago = (Utc::now() - created_at).num_days();
            let days_ago_text = pluralizer::pluralize("day", days_ago as isize, true);
            let mut line = "".to_string();
            let title = title.replace('"', "\\\"");
            line.push_str(&format!(
                "{{{{% details \"{title} ({days_ago_text} old)\" %}}}}\n"
            ));
            labels.into_iter().for_each(|label| {
                line.push_str("* _");
                line.push_str(&label.name);
                line.push('_');
                if let Some(d) = label.description {
                    line.push_str(" - ");
                    line.push_str(&d);
                }
                line.push('\n');
            });
            line.push_str(&format!("\n[Open PR #{number}]({html_url})\n\n"));
            line.push_str("{{% /details %}}\n");
            index.push_str(&line);
        }

        index.push_str(&format!(
            "

## About releases.rs

- [Github Repo](https://github.com/releases-rs/releases-rs/)
- Generated at <span class=\"utc-timestamp\" data-utc=\"{}\">...</span>

",
            Utc::now().to_rfc3339()
        ));

        index
    }
}