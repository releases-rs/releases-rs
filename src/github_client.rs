use crate::config::Config;
use anyhow::Result;
use octocrab::models::issues::Issue;
use octocrab::models::{IssueId, Milestone};
use octocrab::params::issues::Sort;
use octocrab::params::{issues, Direction, State};
use octocrab::Octocrab;
use semver::Version;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GitHubClient {
    octocrab: Octocrab,
    config: Config,
}

impl GitHubClient {
    pub fn new(config: Config) -> Self {
        let mut builder = Octocrab::builder();
        let token = std::env::var("GITHUB_TOKEN").ok();
        if let Some(token) = token {
            builder = builder.personal_token(token);
        }

        Self {
            octocrab: builder.build().unwrap(),
            config,
        }
    }

    pub async fn fetch_milestones(&self) -> Result<HashMap<Version, Milestone>> {
        let mut milestones = HashMap::new();
        let mut issues_page = self.octocrab
            .issues(&self.config.repo_owner, &self.config.repo_name)
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
                    .and_then(|m| Version::parse(&m.title).ok())
                {
                    milestones.entry(version).or_insert(issue.milestone.clone());

                    if milestones.len() > self.config.num_versions {
                        break 'issues_pages;
                    };
                }
            }
            issues_page = match self.octocrab.get_page::<Issue>(&issues_page.next).await? {
                Some(next_page) => next_page,
                None => break,
            };
        }

        Ok(milestones.into_iter().filter_map(|(k, v)| v.map(|milestone| (k, milestone))).collect())
    }

    pub async fn fetch_stabilization_prs(&self) -> Result<HashMap<IssueId, Issue>> {
        let mut stabilization_prs = HashMap::new();

        for search_term in &self.config.stabilization_search_terms {
            println!("search for {search_term} PRs");

            let mut prs_page = self.octocrab
                .search()
                .issues_and_pull_requests(&format!(
                    "is:pr is:open in:title repo:{}/{} {search_term}",
                    self.config.repo_owner, self.config.repo_name
                ))
                .sort("created_at")
                .order("desc")
                .send()
                .await?;

            loop {
                for pr in &prs_page {
                    let title = pr.title.to_lowercase();
                    if title.starts_with(search_term)
                        || title.starts_with(&format!("partial {search_term}"))
                        || title.starts_with(&format!("partially {search_term}"))
                    {
                        stabilization_prs.insert(pr.id, pr.clone());
                    }
                }
                prs_page = match self.octocrab.get_page::<Issue>(&prs_page.next).await? {
                    Some(next_page) => next_page,
                    None => break,
                };
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }

        Ok(stabilization_prs)
    }

    pub async fn fetch_milestone_issues(&self, milestone_id: i64) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut issues_page = self.octocrab
            .issues(&self.config.repo_owner, &self.config.repo_name)
            .list()
            .milestone(issues::Filter::Matches(milestone_id.try_into().expect("overflow in milestone_id")))
            .labels(&vec![String::from("relnotes")])
            .per_page(255)
            .sort(Sort::Created)
            .direction(Direction::Ascending)
            .state(State::Closed)
            .send()
            .await?;

        loop {
            all_issues.extend(issues_page.items.clone());
            issues_page = match self.octocrab.get_page::<Issue>(&issues_page.next).await? {
                Some(next_page) => next_page,
                None => break,
            };
        }

        Ok(all_issues)
    }
}