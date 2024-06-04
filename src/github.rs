use std::path::PathBuf;

use anyhow::Error;
use reqwest::Client;
use serde_json::Value;
use tracing::{error, info};

pub struct Github {
    client: Client,
    github_token: String,
}

impl Github {
    pub fn new(github_token: String) -> Self {
        Github {
            client: Client::new(),
            github_token,
        }
    }

    pub async fn fork_repo(&self, repo_url: &str) -> Result<String, Error> {
        let repo_name = repo_url.split('/').last().unwrap();
        let repo_owner = repo_url.split('/').nth(3).unwrap(); // Assuming URL is in the format https://github.com/{owner}/{repo}

        if repo_owner == "kodylow" {
            info!("Repository belongs to 'kodylow', no need to fork.");
            return Ok(repo_url.to_string());
        }

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

    pub async fn clone_repo(&self, repo_url: &str, work_dir: &PathBuf) {
        let repo_name = repo_url.split("/").last().unwrap();
        let repo_path = work_dir.join(repo_name);
        println!("Cloning repository: {}", repo_url);
        match git2::Repository::clone(repo_url, &repo_path) {
            Ok(_) => println!("Repository cloned successfully."),
            Err(e) => println!("Failed to clone repository: {}", e),
        }
    }
}
