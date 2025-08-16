use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct Config {
    pub num_versions: usize,
    pub rust_releases_url: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub epoch_date: NaiveDate,
    pub hugo_template_dir: String,
    pub hugo_content_dir: String,
    pub hugo_public_dir: String,
    pub stabilization_search_terms: Vec<&'static str>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            num_versions: 5,
            rust_releases_url: "https://raw.githubusercontent.com/rust-lang/rust/stable/RELEASES.md".to_string(),
            repo_owner: "rust-lang".to_string(),
            repo_name: "rust".to_string(),
            epoch_date: NaiveDate::from_ymd_opt(2015, 12, 10).unwrap(),
            hugo_template_dir: "hugo/rust-changelogs/template".to_string(),
            hugo_content_dir: "hugo/rust-changelogs/content".to_string(),
            hugo_public_dir: "hugo/rust-changelogs/public".to_string(),
            stabilization_search_terms: vec!["stabilise", "stabilize", "stabilisation", "stabilization"],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}