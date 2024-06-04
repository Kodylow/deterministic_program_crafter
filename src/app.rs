use std::path::PathBuf;

use git2::Repository;
use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info};

use crate::config;
use crate::groq::Groq;

#[derive(Deserialize)]
struct Crate {
    crate_id: String,
    repository: Option<String>,
}

pub struct App {
    instructions: String,
    work_dir: PathBuf,
    crate_tool: Option<String>,
    groq: Groq,
    cargo_cookie: String,
    client: Client,
}

impl App {
    pub async fn new(cli_args: &config::CliArgs) -> App {
        let groq = Groq::new(&cli_args.groq_api_key);
        let client = Client::new();

        // Ensure the work directory exists
        if !cli_args.work_dir.exists() {
            std::fs::create_dir_all(&cli_args.work_dir).expect("Failed to create work directory");
        }

        App {
            instructions: cli_args.instructions.clone(),
            work_dir: cli_args.work_dir.clone(),
            crate_tool: None,
            groq,
            cargo_cookie: cli_args.cargo_cookie.clone(),
            client,
        }
    }

    pub async fn run(&self) -> Result<(), anyhow::Error> {
        let keywords = self.identify_tool().await?;

        if let Some(keywords) = keywords {
            if let Some(crate_tool) = self.search_crates_io(&keywords).await {
                self.clone_repo(&crate_tool).await;
            } else {
                println!("Failed to find crate on crates.io");
                return Err(anyhow::anyhow!("Failed to find crate on crates.io"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to identify tool"));
        }

        Ok(())
    }

    async fn identify_tool(&self) -> Result<Option<String>, anyhow::Error> {
        let crates = self.groq.get_crates_list(&self.instructions).await?;
        info!("Tools identified: {}", crates.join(", "));
        let first_tool = crates.first().cloned();
        info!(
            "First tool: {}",
            first_tool.clone().unwrap_or("None".to_string())
        );
        Ok(first_tool)
    }

    async fn search_crates_io(&self, crate_name: &str) -> Option<String> {
        info!("Searching crates.io for {}", crate_name);
        let request_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        let client = self.client.clone();
        let cookie = format!("cargo_session={}", self.cargo_cookie);

        let response = client
            .get(&request_url)
            .header(reqwest::header::COOKIE, cookie)
            .header(
                reqwest::header::USER_AGENT,
                "my_crawler (help@mycrawler.com)",
            ) // Add this line
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read body".to_string());

                info!("Response Status: {}", status);
                // info!("Response Headers: {:?}", headers);
                // info!("Response Body: {}", body);

                if status.is_success() {
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(crate_data) => {
                            let repo_url = crate_data["crate"]["repository"]
                                .as_str()
                                .map(|s| s.to_string());
                            info!("Repository URL: {:?}", repo_url);
                            repo_url
                        }
                        Err(e) => {
                            error!("Failed to parse JSON response: {}", e);
                            None
                        }
                    }
                } else {
                    error!(
                        "Failed to fetch crate info from crates.io with status: {}",
                        status
                    );
                    None
                }
            }
            Err(e) => {
                error!("Error fetching crate info: {}", e);
                None
            }
        }
    }

    async fn clone_repo(&self, repo_url: &str) {
        println!("Cloning repository: {}", repo_url);
        match Repository::clone(repo_url, &self.work_dir) {
            Ok(_) => println!("Repository cloned successfully."),
            Err(e) => println!("Failed to clone repository: {}", e),
        }
    }
}
