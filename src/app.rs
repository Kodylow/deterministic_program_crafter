use std::path::PathBuf;

use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info};

use crate::config;
use crate::crates_io::CratesIo;
use crate::github::Github;
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
    github: Github,
    crates_io: CratesIo,
}

impl App {
    pub async fn new(cli_args: &config::CliArgs) -> App {
        let groq = Groq::new(&cli_args.groq_api_key);
        let github = Github::new(cli_args.github_token.clone());
        let crates_io = CratesIo::new(cli_args.cargo_cookie.clone());

        // Ensure the work directory exists
        if !cli_args.work_dir.exists() {
            std::fs::create_dir_all(&cli_args.work_dir).expect("Failed to create work directory");
        }

        App {
            instructions: cli_args.instructions.clone(),
            work_dir: cli_args.work_dir.clone(),
            crate_tool: None,
            groq,
            github,
            crates_io,
        }
    }

    pub async fn run(&self) -> Result<(), anyhow::Error> {
        let tool = self.identify_tool().await?;

        if let Some(tool) = tool {
            if let Some(crate_tool) = self.crates_io.search(&tool).await {
                let forked_repo_url = self.github.fork_repo(&crate_tool).await?;
                self.github
                    .clone_repo(&forked_repo_url, &self.work_dir)
                    .await;
            } else {
                error!("Failed to find crate on crates.io");
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
}
