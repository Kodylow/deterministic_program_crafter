use std::path::PathBuf;

use serde::Deserialize;
use tokio::process::Command;
use tracing::{error, info};

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
        let tool = self.identify_and_validate_tool().await?;
        let repo_url = self.find_crate_and_set_repo(tool).await?;
        self.prepare_repository(repo_url).await?;
        self.process_repository_files().await?;
        self.update_and_write_flake().await?;
        self.validate_and_check_program(self.work_dir.join(self.repo_name.as_ref().unwrap()))
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

    async fn identify_and_validate_tool(&self) -> Result<String, anyhow::Error> {
        let tool = self
            .identify_tool()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to identify tool"))?;
        Ok(tool)
    }

    async fn find_crate_and_set_repo(&mut self, tool: String) -> Result<String, anyhow::Error> {
        let repo_url = self
            .crates_io
            .search(&tool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find crate on crates.io"))?;
        self.repo_url = Some(repo_url.clone());
        let repo_name = repo_url.split('/').last().unwrap().to_string();
        self.repo_name = Some(repo_name.clone());
        Ok(repo_url)
    }

    async fn prepare_repository(&mut self, repo_url: String) -> Result<(), anyhow::Error> {
        self.github
            .fork_and_clone(&repo_url, &self.work_dir)
            .await?;
        self.flake = Some(Flake::new(
            &self.repo_name.as_ref().unwrap(),
            &self.work_dir,
        ));
        self.flake
            .as_ref()
            .unwrap()
            .ensure_flake_nix(&PathBuf::from(
                "/Users/kody/Documents/github/deterministic_program_crafter/reference_flake.nix",
            ))
            .await?;
        Ok(())
    }

    async fn process_repository_files(&self) -> Result<(), anyhow::Error> {
        let repo_dir = self.work_dir.join(self.repo_name.as_ref().unwrap());
        let _ = std::fs::read_to_string(&self.flake.as_ref().unwrap().flake_path)?;
        let _ = std::fs::read_to_string(&repo_dir.join("Cargo.toml"))?;
        let _ = std::fs::read_to_string(&repo_dir.join("README.md"))?;
        let _ = std::fs::read_to_string(&repo_dir.join("src/main.rs"))?;
        Ok(())
    }

    async fn update_and_write_flake(&mut self) -> Result<(), anyhow::Error> {
        let repo_dir = self.work_dir.join(self.repo_name.as_ref().unwrap());
        let binary_name = self.repo_name.as_ref().unwrap();
        let cargo_toml_contents = std::fs::read_to_string(&repo_dir.join("Cargo.toml"))?;
        let readme_contents = std::fs::read_to_string(&repo_dir.join("README.md"))?;
        let main_rs_contents = std::fs::read_to_string(&repo_dir.join("src/main.rs"))?;

        let crate_description = self
            .groq
            .create_crate_description(&cargo_toml_contents, &readme_contents, &main_rs_contents)
            .await?;

        self.flake
            .as_ref()
            .unwrap()
            .write_description_and_binary_name(&crate_description, &binary_name)
            .await?;

        Ok(())
    }

    async fn validate_and_check_program(&self, repo_dir: PathBuf) -> Result<bool, anyhow::Error> {
        // Run cargo check initially
        let errors = self.cargo_check(&repo_dir).await?;

        let mut main_rs_contents = std::fs::read_to_string(&repo_dir.join("src/main.rs"))?;

        loop {
            let instructions = self
                .groq
                .validate_binary(&self.instructions, &main_rs_contents, &errors)
                .await?;

            let first_word = instructions.split_whitespace().next().unwrap_or("");

            match first_word {
                "Correct" => {
                    info!("Program satisfies user instructions");
                    return Ok(true);
                }
                _ => {
                    info!("Program does not satisfy user instructions, rewriting code");
                    // Write the rewritten code to the main_rs_contents for
                    main_rs_contents = self
                        .write_code(instructions, repo_dir.join("src/main.rs"), &repo_dir)
                        .await?;
                    // Run cargo check after each rewrite
                    self.cargo_check(&repo_dir).await?;
                }
            }
        }
    }

    async fn cargo_check(&self, repo_dir: &PathBuf) -> Result<String, anyhow::Error> {
        info!("Running cargo check...");
        let output = Command::new("cargo")
            .arg("check")
            .current_dir(repo_dir)
            .output()
            .await?;

        if !output.status.success() {
            let errors = String::from_utf8_lossy(&output.stderr);
            error!("Cargo check failed: {}", errors);
            return Err(anyhow::anyhow!("Cargo check failed: {}", errors));
        }

        info!("Cargo check passed");
        Ok("None".to_string())
    }

    async fn write_code(
        &self,
        instructions: String,
        main_rs_path: PathBuf,
        repo_dir: &PathBuf,
    ) -> Result<String, anyhow::Error> {
        let main_rs_contents = std::fs::read_to_string(&main_rs_path)?;

        let new_contents = self
            .groq
            .rewrite_main_rs(&instructions, &main_rs_contents)
            .await?;

        // add cargo deps
        self.groq.add_cargo_deps(&new_contents, &repo_dir).await?;

        // Remove Markdown code block indicators and any leading text before the code
        // starts
        let cleaned_contents = new_contents
            .lines()
            .filter(|line| {
                !line.starts_with("```") && !line.contains("Here is the rewritten main.rs file:")
            })
            .collect::<Vec<&str>>()
            .join("\n");

        std::fs::write(&main_rs_path, cleaned_contents.clone())?;
        Ok(cleaned_contents)
    }
}
