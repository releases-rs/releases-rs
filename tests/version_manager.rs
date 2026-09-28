
use semver::Version;
use rust_changelogs::{Config, VersionManager};
use itertools::Itertools;
use chrono::NaiveDate;

#[test]
fn version_weights() {
    let config = Config::new();
    let version_manager = VersionManager::new(config);

    let versions = vec![
        Version::parse("1.90.0").unwrap(),
        Version::parse("1.85.1").unwrap(),
        Version::parse("1.1.0").unwrap(),
        Version::parse("1.0.0").unwrap(),
        Version::parse("1.0.0-alpha.2").unwrap(),
        Version::parse("1.0.0-alpha").unwrap(),
        Version::parse("0.12.0").unwrap(),
    ];

    let weights: Vec<_> = versions.iter()
        .map(|v| (v, version_manager.determine_weight(v)))
        .sorted_by(|a, b| a.1.cmp(&b.1))
        .map(|(v, _)| v)
        .collect();

    for (index, version) in weights.into_iter().enumerate() {
        assert_eq!(version, &versions[index]);
    }
}

fn changelog(version: &str, date: &str) -> String {
    format!("Version {version} ({date})\n==========================\n\n- Notes for {version}\n\n")
}

fn date(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

#[test]
fn merge_changelogs_combines_sources() {
    let version_manager = VersionManager::new(Config::new());

    // `master` lacks a point release that only landed on `stable`
    let master = [changelog("1.81.0", "2024-09-05"), changelog("1.80.0", "2024-07-25")].concat();
    // `stable` lacks the latest releases entirely
    let stable = [changelog("1.80.1", "2024-08-08"), changelog("1.80.0", "2024-07-25")].concat();

    let changelogs = version_manager.merge_changelogs([
        version_manager.parse_changelogs(&master),
        version_manager.parse_changelogs(&stable),
    ]);

    let versions: Vec<_> = changelogs.keys().cloned().sorted().collect();
    assert_eq!(
        versions,
        vec![
            Version::parse("1.80.0").unwrap(),
            Version::parse("1.80.1").unwrap(),
            Version::parse("1.81.0").unwrap(),
        ]
    );
    assert_eq!(changelogs[&Version::parse("1.81.0").unwrap()].1, date("2024-09-05"));
}

#[test]
fn scheduled_release_dates() {
    let version_manager = VersionManager::new(Config::new());

    assert_eq!(version_manager.scheduled_release_date(5), date("2015-12-10"));
    assert_eq!(version_manager.scheduled_release_date(90), date("2025-09-18"));
    assert_eq!(version_manager.scheduled_release_date(98), date("2026-08-20"));

    assert_eq!(version_manager.scheduled_stable_minor(date("2026-08-19")), 97);
    assert_eq!(version_manager.scheduled_stable_minor(date("2026-08-20")), 98);
    assert_eq!(version_manager.scheduled_stable_minor(date("2026-09-30")), 98);
    assert_eq!(version_manager.scheduled_stable_minor(date("2026-10-01")), 99);
}

#[test]
fn current_versions_from_release_notes() {
    let version_manager = VersionManager::new(Config::new());
    let notes = [
        changelog("1.99.0", "2026-10-01"),
        changelog("1.98.1", "2026-09-03"),
        changelog("1.98.0", "2026-08-20"),
    ]
    .concat();
    let changelogs = version_manager.parse_changelogs(&notes);

    let (stable, beta, nightly) = version_manager.get_current_versions(&changelogs, date("2026-09-28"));
    assert_eq!(stable, Version::parse("1.98.1").unwrap());
    assert_eq!(beta, Version::parse("1.99.0").unwrap());
    assert_eq!(nightly, Version::parse("1.100.0").unwrap());
}

#[test]
fn current_versions_when_release_notes_lag_behind() {
    let version_manager = VersionManager::new(Config::new());
    let notes = [changelog("1.97.1", "2026-07-16"), changelog("1.97.0", "2026-07-09")].concat();
    let changelogs = version_manager.parse_changelogs(&notes);

    let (stable, beta, nightly) = version_manager.get_current_versions(&changelogs, date("2026-09-28"));
    assert_eq!(stable, Version::parse("1.98.0").unwrap());
    assert_eq!(beta, Version::parse("1.99.0").unwrap());
    assert_eq!(nightly, Version::parse("1.100.0").unwrap());
}
