use anyhow::Result;
use chrono::Utc;
use rust_changelogs::{ChangelogGenerator, Config, GitHubClient, HugoManager, VersionManager};
use std::collections::HashSet;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::new();
    let version_manager = VersionManager::new(config.clone());
    let github_client = GitHubClient::new(config.clone());
    let changelog_generator = ChangelogGenerator::new(version_manager.clone());
    let hugo_manager = HugoManager::new(config.clone());

    hugo_manager.setup_directories()?;

    let body = reqwest::get(&config.rust_releases_url)
        .await?
        .error_for_status()?
        .text()
        .await?;

    let changelogs = version_manager.parse_changelogs(&body);

    for (version, (changelog, release_date)) in changelogs.iter() {
        let content = changelog_generator.generate_released_version_content(version, changelog, release_date);
        hugo_manager.write_version_file(version, &content)?;
    }

    let milestones = github_client.fetch_milestones().await?;
    let stabilization_prs = github_client.fetch_stabilization_prs().await?;

    let released_versions: HashSet<_> = changelogs
        .iter()
        .filter(|(_, (_, date))| *date <= Utc::now().naive_utc().date())
        .map(|(k, _)| k.clone())
        .collect();

    let issues_versions: HashSet<_> = milestones.keys().cloned().collect();
    let unreleased_versions: HashSet<_> = issues_versions.difference(&released_versions).collect();

    let unreleased_version_to_milestone: Vec<_> = milestones
        .into_iter()
        .filter(|(v, _m)| unreleased_versions.contains(v))
        .map(|(v, m)| (v, m.number))
        .collect();

    let (stable_version, beta_version, nightly_version) = version_manager.get_current_versions(&changelogs);

    for (unreleased_version, milestone_id) in unreleased_version_to_milestone.iter() {
        let issues = github_client.fetch_milestone_issues(*milestone_id).await?;
        let changelog = changelog_generator.generate_unreleased_version_content(
            unreleased_version, 
            *milestone_id, 
            &stable_version, 
            &issues
        );

        if !changelogs.contains_key(unreleased_version) {
            hugo_manager.write_version_file(unreleased_version, &changelog)?;
        }
    }

    let index_content = changelog_generator.generate_index_content(
        &stable_version, 
        &beta_version, 
        &nightly_version, 
        &unreleased_versions, 
        stabilization_prs
    );
    hugo_manager.write_index_file(&index_content)?;

    hugo_manager.build_site()?;

    Ok(())
}