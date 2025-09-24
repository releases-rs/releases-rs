
use semver::Version;
use rust_changelogs::{Config, VersionManager};

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

    let mut weights: Vec<_> = versions.iter()
        .map(|v| (v, version_manager.determine_weight(v)))
        .collect();

    // Sort like this to match the site
    weights.sort_by(|a, b| a.1.cmp(&b.1));

    for (index, (version, _)) in weights.into_iter().enumerate() {
        assert_eq!(version, &versions[index]);
    }
}
