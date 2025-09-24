use crate::config::Config;
use chrono::{Duration, NaiveDate, Utc};
use regex::RegexBuilder;
use semver::Version;
use std::collections::HashMap;
use tap::Tap;

#[derive(Debug, Clone)]
pub struct ReleaseDate {
    pub release_date: NaiveDate,
    pub branch_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct VersionManager {
    config: Config,
}

// We do this because of https://github.com/rust-lang/rust/commit/495d7ee587dc1b8d99fd9f0bce2f72b0072e3aca
// So we'll be a bit permissive with such cases where the minor version is missing
fn parse_lenient_version(version: &str) -> Option<Version> {
    let splits = version.split('.').count();

    if splits < 3 {
        return Version::parse(&format!("{version}.0")).ok()
    }

    Version::parse(version).ok()
}

impl VersionManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn calculate_release_date(&self, now_date: NaiveDate, incr: u32) -> ReleaseDate {
        let new_releases = ((now_date - self.config.epoch_date).num_weeks() as f64 / 6.0).floor() as u32;
        let release_date = self.config.epoch_date + Duration::weeks(((new_releases + incr) * 6).into());
        let branch_date = self.config.epoch_date + Duration::weeks(((new_releases + incr - 1) * 6).into()) - Duration::days(6);

        ReleaseDate {
            release_date,
            branch_date,
        }
    }

    pub fn determine_weight(&self, version: &Version) -> u64 {
        let base_weight = u64::MAX - ((version.major) << 24) - ((version.minor) << 8) - version.patch;

        if version.pre.is_empty() {
            return base_weight
        }

        // Very hacky, but works for now
        let pre_hash = version.pre.to_string().chars().map(|c| c as u64).sum::<u64>() % 100;
        base_weight.saturating_add(pre_hash)
    }

    pub fn parse_changelogs(&self, body: &str) -> HashMap<Version, (String, NaiveDate)> {
        let split_re = RegexBuilder::new("^Version\\s+")
            .multi_line(true)
            .build()
            .unwrap();
            
        split_re
            .split(body)
            .filter_map(|s| {
                if let Some(ws_idx) = s.find(|c: char| c.is_whitespace()) {
                    let rest = &s[ws_idx..];
                    let version = &s[0..ws_idx];
                    let time: NaiveDate = s[ws_idx + 1..].trim_start()[1..11].parse().unwrap();
                    if let Some(version) = parse_lenient_version(version) {
                        let changelog = rest.lines().skip(2).collect::<Vec<_>>().join("\n");
                        Some((version, (changelog, time)))
                    } else {
                        panic!("Lenient version parsing failed for '{version}'");
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_current_versions(&self, changelogs: &HashMap<Version, (String, NaiveDate)>) -> (Version, Version, Version) {
        let stable_version = changelogs
            .iter()
            .filter(|(_, (_, release_date))| *release_date <= Utc::now().naive_utc().date())
            .max_by_key(|(v, _)| *v)
            .unwrap()
            .0
            .clone();
            
        let beta_version = stable_version.clone().tap_mut(|v| {
            v.minor += 1;
            v.patch = 0;
        });
        
        let nightly_version = stable_version.clone().tap_mut(|v| {
            v.minor += 2;
            v.patch = 0;
        });

        (stable_version, beta_version, nightly_version)
    }
}
