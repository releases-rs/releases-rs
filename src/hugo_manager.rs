use crate::config::Config;
use anyhow::Result;
use fs_extra::dir::CopyOptions;
use semver::Version;
use std::path::Path;
use std::{fs, io};

#[derive(Debug)]
pub struct HugoManager {
    config: Config,
}

impl HugoManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn setup_directories(&self) -> Result<()> {
        let _ = fs::remove_dir_all(&self.config.hugo_content_dir);
        let _ = remove_dir_contents(&self.config.hugo_public_dir);
        
        let mut options = CopyOptions::new();
        options.copy_inside = true;
        fs_extra::dir::copy(&self.config.hugo_template_dir, &self.config.hugo_content_dir, &options)
            .expect("copy hugo dir");
        
        Ok(())
    }

    pub fn write_version_file(&self, version: &Version, content: &str) -> Result<()> {
        fs::write(format!("{}/docs/{version}.md", self.config.hugo_content_dir), content)?;
        Ok(())
    }

    pub fn write_index_file(&self, content: &str) -> Result<()> {
        fs::write(format!("{}/_index.md", self.config.hugo_content_dir), content)?;
        Ok(())
    }

    pub fn build_site(&self) -> Result<()> {
        let res = std::process::Command::new("hugo")
            .arg("--minify")
            .arg("--logLevel")
            .arg("debug")
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
}

fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        fs::remove_file(entry?.path())?;
    }
    Ok(())
}