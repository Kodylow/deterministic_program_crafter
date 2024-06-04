use std::path::PathBuf;

use serde::Deserialize;
use tracing::info;

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
    repo_url: Option<String>,
    repo_name: Option<String>,
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
            repo_url: None,
            repo_name: None,
            groq,
            github,
            crates_io,
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

        self.ensure_flake_nix(&repo_name).await?;

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

    async fn ensure_flake_nix(&self, crate_name: &str) -> Result<(), anyhow::Error> {
        let flake_path = self.work_dir.join(crate_name).join("flake.nix");
        let reference_flake_path = PathBuf::from(
            "/Users/kody/Documents/github/deterministic_program_crafter/reference_flake.nix",
        );
        info!("Reference flake path: {}", reference_flake_path.display());

        if !reference_flake_path.exists() {
            info!(
                "Reference flake.nix not found at {}",
                reference_flake_path.display()
            );
            return Ok(());
        }

        if flake_path.exists() {
            info!("Found a flake.nix at {}", flake_path.display());
        } else {
            info!("Creating flake.nix at {}", flake_path.display());
            let contents = std::fs::read_to_string(&reference_flake_path).map_err(|e| {
                info!("Failed to read reference flake.nix: {}", e);
                e
            })?;
            std::fs::write(&flake_path, contents).map_err(|e| {
                info!(
                    "Failed to write to flake.nix at {}: {}",
                    flake_path.display(),
                    e
                );
                e
            })?;
        }
        Ok(())
    }
}
