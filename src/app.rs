use std::path::PathBuf;

use git2::Repository;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;

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

        Ok(crates.first().cloned())
    }

    async fn search_crates_io(&self, keywords: &str) -> Option<String> {
        let request_url = format!("https://crates.io/api/v1/crates/{}", keywords);
        let response = self.client.get(&request_url).send().await.unwrap();

        if response.status().is_success() {
            let crate_data: Crate = response.json().await.unwrap();
            crate_data.repository
        } else {
            println!("Failed to fetch data for {}", keywords);
            None
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
