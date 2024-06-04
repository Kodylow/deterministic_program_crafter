use std::path::PathBuf;

use serde::Deserialize;
use tracing::info;

use crate::config;
use crate::crates_io::CratesIo;
use crate::flake::Flake;
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
    repo_url: Option<String>,
    repo_name: Option<String>,
    groq: Groq,
    github: Github,
    crates_io: CratesIo,
    flake: Option<Flake>,
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
            repo_url: None,
            repo_name: None,
            groq,
            github,
            crates_io,
            flake: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let tool = self
            .identify_tool()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to identify tool"))?;

        let repo_url = self
            .crates_io
            .search(&tool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find crate on crates.io"))?;

        self.repo_url = Some(repo_url.clone());
        let repo_name = repo_url.split('/').last().unwrap().to_string();
        self.repo_name = Some(repo_name.clone());

        self.github
            .fork_and_clone(&repo_url, &self.work_dir)
            .await?;

        self.flake = Some(Flake::new(&repo_name, &self.work_dir));
        self.flake
            .as_ref()
            .unwrap()
            .ensure_flake_nix(&PathBuf::from(
                "/Users/kody/Documents/github/deterministic_program_crafter/reference_flake.nix",
            ))
            .await?;

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
