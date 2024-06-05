use std::fs::remove_dir_all;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Error;
use reqwest::Client;
use serde_json::Value;
use tracing::{error, info};

use crate::groq::Groq;

pub struct Github {
    client: Client,
    github_token: String,
    repo_url: String,
}

impl Github {
    pub fn new(github_token: String) -> Self {
        Github {
            client: Client::new(),
            github_token,
            repo_url: String::new(),
        }
    }

    pub async fn fork_and_clone(
        &mut self,
        crate_tool: &str,
        work_dir: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        let forked_repo_url = self.fork_repo(crate_tool).await?;
        self.repo_url = forked_repo_url.clone();
        self.clone_repo(&forked_repo_url, work_dir).await
    }

    pub async fn fork_repo(&self, repo_url: &str) -> Result<String, Error> {
        let repo_name = repo_url.split('/').last().unwrap();
        let repo_owner = repo_url.split('/').nth(3).unwrap(); // Assuming URL is in the format https://github.com/{owner}/{repo}

        // if repo_owner == "kodylow" {
        //     info!("Repository belongs to 'kodylow', no need to fork.");
        //     return Ok(repo_url.to_string());
        // }

        let user_repos_url = format!("https://api.github.com/user/repos");
        let repos_response = self
            .client
            .get(&user_repos_url)
            .bearer_auth(&self.github_token)
            .send()
            .await?;

        if repos_response.status().is_success() {
            let repos: Vec<Value> = repos_response.json().await?;
            if let Some(fork) = repos.iter().find(|repo| {
                repo["fork"].as_bool() == Some(true)
                    && repo["parent"]["full_name"].as_str() == Some(repo_name)
            }) {
                let forked_repo_url = fork["clone_url"].as_str().unwrap().to_string();
                return Ok(forked_repo_url);
            }
        } else {
            error!("Failed to retrieve user repositories");
            return Err(anyhow::anyhow!("Failed to retrieve user repositories"));
        }

        let fork_url = format!("https://api.github.com/repos/{}/forks", repo_name);
        let response = self
            .client
            .post(&fork_url)
            .bearer_auth(&self.github_token)
            .send()
            .await?;

        if response.status().is_success() {
            let forked_repo: Value = response.json().await?;
            let forked_repo_url = forked_repo["clone_url"].as_str().unwrap().to_string();
            return Ok(forked_repo_url);
        } else {
            return Err(anyhow::anyhow!("Failed to fork repository"));
        }
    }

    pub async fn clone_repo(
        &self,
        repo_url: &str,
        work_dir: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        let repo_name = repo_url.split("/").last().unwrap();
        let repo_path = work_dir.join(repo_name);

        // Check if the directory exists and delete it if it does
        if repo_path.exists() {
            info!("Deleting existing directory: {}", repo_path.display());
            remove_dir_all(&repo_path)
                .map_err(|e| anyhow::anyhow!("Failed to delete existing directory: {}", e))?;
        }

        info!("Cloning repository: {}", repo_url);
        match git2::Repository::clone(repo_url, &repo_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to clone repository: {}", e)),
        }
    }

    pub async fn create_branch(&self, repo_dir: &PathBuf, branch_name: &str) -> Result<(), Error> {
        info!("Creating branch: {}", branch_name);
        let status = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(repo_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to create branch: {}", branch_name));
        }
        Ok(())
    }

    pub async fn push_changes(&self, repo_dir: &PathBuf) -> Result<(), Error> {
        info!("Pushing changes to remote repository...");
        let status = Command::new("git")
            .args(["push", "-f"])
            .current_dir(repo_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to push changes"));
        }
        Ok(())
    }

    pub async fn open_pull_request(&self, repo_dir: &PathBuf, groq: &Groq) -> Result<(), Error> {
        info!("Opening pull request...");

        let current_branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo_dir)
            .output()?;
        let current_branch = String::from_utf8(current_branch_output.stdout)?
            .trim()
            .to_string();
        info!("Current branch: {}", current_branch);

        // Ensure changes are pushed to the remote repository
        let push_status = Command::new("git")
            .args(["push", "--set-upstream", "origin", &current_branch])
            .current_dir(repo_dir)
            .status()?;
        if !push_status.success() {
            return Err(anyhow::anyhow!(
                "Failed to push changes to remote repository"
            ));
        }

        // Log diff to ensure there are changes
        let git_diff = Command::new("git")
            .args(["diff", "HEAD~1..HEAD"])
            .current_dir(repo_dir)
            .output()?
            .stdout;
        let git_diff_str = String::from_utf8(git_diff)?;

        // Generate PR message and title
        let (pr_title, pr_message) = groq
            .generate_pr_message_and_title(&self.github_token, &git_diff_str)
            .await?;

        let status = Command::new("gh")
            .args([
                "pr",
                "create",
                "--head",
                &format!("{}", current_branch),
                "--title",
                &pr_title,
                "--body",
                &pr_message,
            ])
            .current_dir(repo_dir)
            .status();

        match status {
            Ok(status) if status.success() => {
                info!("Pull request opened successfully");
            }
            Ok(_) | Err(_) => {
                error!("Failed to open pull request, but continuing execution");
            }
        }

        Ok(())
    }
}
