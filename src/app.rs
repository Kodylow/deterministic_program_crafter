use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use fs_extra::dir;
use fs_extra::dir::CopyOptions;
use serde::Deserialize;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info};

use crate::config;
use crate::crates_io::CratesIo;
use crate::flake::Flake;
use crate::github::Github;
use crate::groq::Groq;

// macro_rules! wait_for_enter {
//     () => {{
//         use tokio::io::{self, AsyncBufReadExt, BufReader};
//         let mut reader = BufReader::new(io::stdin());
//         let mut pause = String::new();
//         println!("Press ENTER to continue...");
//         reader.read_line(&mut pause).await?;
//     }};
// }

#[derive(Deserialize)]
struct Crate {
    _crate_id: String,
    _repository: Option<String>,
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
        // First PR: flake.nix
        self.update_and_write_flake().await?;
        self.flake.as_ref().unwrap().check_flake_nix().await?;
        self.push_changes(false).await?;
        self.github
            .open_pull_request(
                &self.work_dir.join(self.repo_name.as_ref().unwrap()),
                &self.groq,
            )
            .await?;
        // Second PR: flakebox
        self.github
            .create_branch(
                &self.work_dir.join(self.repo_name.as_ref().unwrap()),
                "flakebox",
            )
            .await?;
        self.install_flakebox_files(&self.work_dir.join(self.repo_name.as_ref().unwrap()))
            .await?;
        self.push_changes(false).await?;
        self.github
            .open_pull_request(
                &self.work_dir.join(self.repo_name.as_ref().unwrap()),
                &self.groq,
            )
            .await?;

        // Third PR: main.rs updates
        self.github
            .create_branch(
                &self.work_dir.join(self.repo_name.as_ref().unwrap()),
                "new-feature",
            )
            .await?;
        self.validate_and_check_program(self.work_dir.join(self.repo_name.as_ref().unwrap()))
            .await?;
        let binary_path = self.build_and_output_binary().await?;
        self.run_binary(&binary_path).await?;
        self.push_changes(true).await?;
        self.github
            .open_pull_request(
                &self.work_dir.join(self.repo_name.as_ref().unwrap()),
                &self.groq,
            )
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
        let repo_dir = self.work_dir.join(self.repo_name.as_ref().unwrap());
        self.github
            .fork_and_clone(&repo_url, &self.work_dir)
            .await?;
        self.github.create_branch(&repo_dir, "flakebot").await?;
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

        // Modify .gitignore file
        info!("Modifying .gitignore file...");
        let gitignore_path = repo_dir.join(".gitignore");
        let gitignore_contents = "/target\n/result\n/work_dir\n/db\n/tmp\n/nix\n/result\n";
        if !gitignore_path.exists() {
            tokio::fs::File::create(&gitignore_path).await?;
        }
        tokio::fs::write(&gitignore_path, gitignore_contents).await?;
        Ok(())
    }

    async fn commit_changes(&self, main_diff: bool) -> Result<(), anyhow::Error> {
        info!("Staging changes...");
        let repo_dir = self.work_dir.join(self.repo_name.as_ref().unwrap());

        let status = Command::new("git")
            .arg("add")
            .arg("--all")
            .current_dir(&repo_dir)
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to add changes"));
        }

        // Generate the commit message
        let mut git_diff_command = {
            let mut cmd = Command::new("git");
            cmd.arg("diff").arg("--cached");
            if main_diff {
                cmd.arg("src/main.rs"); // Targeting only main.rs
            }
            cmd
        };

        let git_diff = git_diff_command.output().await?.stdout;
        let git_diff_str = String::from_utf8(git_diff)?;
        let commit_message = self.groq.generate_commit_message(&git_diff_str).await?;

        info!("Committing changes...");
        let status = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(commit_message)
            .arg("--author")
            .arg("FlakeBot <flakebot@flakebot.com>")
            .arg("--no-verify")
            .current_dir(&repo_dir)
            .status()
            .await?;

        if !status.success() {
            error!("Nothing to commit");
        }

        Ok(())
    }

    pub async fn push_changes(&self, main_diff: bool) -> Result<(), anyhow::Error> {
        self.commit_changes(main_diff).await?;
        self.github
            .push_changes(&self.work_dir.join(self.repo_name.as_ref().unwrap()))
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

        let initial_main_rs_contents = std::fs::read_to_string(&repo_dir.join("src/main.rs"))?;
        let mut main_rs_contents = initial_main_rs_contents.clone();

        loop {
            let instructions = self
                .groq
                .validate_binary(&self.instructions, &main_rs_contents)
                .await?;

            let first_word = instructions.split_whitespace().next().unwrap_or("");

            match first_word {
                "Correct" => {
                    info!("Program satisfies user instructions");
                    return Ok(true);
                }
                _ => {
                    info!("Program does not satisfy user instructions, rewriting code");
                    main_rs_contents = self
                        .write_code(instructions, repo_dir.join("src/main.rs"), &repo_dir)
                        .await?;

                    // Attempt to run cargo check after each rewrite
                    let check_result = self.cargo_check(&repo_dir).await;
                    if check_result.is_err() {
                        error!("Cargo check failed, retrying with incremental fixes...");
                        main_rs_contents = initial_main_rs_contents.clone();
                        std::fs::write(&repo_dir.join("src/main.rs"), &main_rs_contents)?;
                    }
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
        let _ = self
            .groq
            .add_cargo_deps(&new_contents, &repo_dir)
            .await
            .map_err(|e| {
                error!("Failed to add cargo dependencies: {}", e);
            });

        // Remove Markdown code block indicators and any leading text before the code
        // starts
        let cleaned_contents = new_contents
            .lines()
            .filter(|line| !line.starts_with("```") && !line.contains("main.rs"))
            .collect::<Vec<&str>>()
            .join("\n");

        std::fs::write(&main_rs_path, cleaned_contents.clone())?;
        Ok(cleaned_contents)
    }

    async fn build_and_output_binary(&self) -> Result<PathBuf, anyhow::Error> {
        let repo_dir = self.work_dir.join(self.repo_name.as_ref().unwrap());
        let output_path = repo_dir.join("result"); // This is where nix-build outputs the binary by default

        info!("Building the tool using flake.nix...");
        let crate_name = self.repo_name.as_ref().unwrap(); // Assuming repo_name holds the CRATE_NAME
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "cd {} && nix build && sha256sum ./result/bin/{}",
                repo_dir.display(),
                crate_name
            ))
            .output()
            .await?;
        info!("nix-build output sha256sum: {:?}", output);

        if !output.status.success() {
            let errors = String::from_utf8_lossy(&output.stderr);
            error!("nix-build failed: {}", errors);
            return Err(anyhow::anyhow!("nix-build failed: {}", errors));
        }

        info!("Build successful, binary located at {:?}", output_path);

        // Optionally, you can copy the binary to a specific location
        let desired_output_path = self.work_dir.join("final_binary");
        std::fs::copy(
            output_path
                .join("bin")
                .join(self.repo_name.as_ref().unwrap()),
            &desired_output_path,
        )?;

        // Set the binary as executable
        std::fs::set_permissions(&desired_output_path, std::fs::Permissions::from_mode(0o755))?;

        Ok(desired_output_path)
    }

    pub async fn run_binary(&self, binary_path: &PathBuf) -> Result<(), anyhow::Error> {
        info!("Making sure the binary is executable...");
        let output = Command::new("chmod")
            .arg("+x")
            .arg(binary_path)
            .output()
            .await?;

        if !output.status.success() {
            let errors = String::from_utf8_lossy(&output.stderr);
            error!("Failed to make binary executable: {}", errors);
            return Err(anyhow::anyhow!(
                "Failed to make binary executable: {}",
                errors
            ));
        }

        info!("Running the binary...");
        let mut child = Command::new(binary_path)
            .spawn()
            .expect("Failed to start binary as a separate process");

        info!("Binary started successfully, server running in a separate process...");

        // Print interaction instructions
        self.print_interaction_instructions().await?;

        // Execute curl commands loop
        self.execute_curl_commands().await?;

        // Kill the process after finishing with curl commands
        child.kill().await?;
        info!("Process killed successfully.");

        Ok(())
    }

    pub async fn print_interaction_instructions(&self) -> Result<(), anyhow::Error> {
        let main_rs_path = self
            .work_dir
            .join(self.repo_name.as_ref().unwrap())
            .join("src/main.rs");
        let instructions = self
            .groq
            .get_interaction_instructions(&main_rs_path)
            .await?;

        info!(
            "\n**************************\n\
            Tool Interaction Instructions: \n\
            ***************************\n\
            {}\n\
            ***************************\n\n",
            instructions
        );
        Ok(())
    }

    pub async fn execute_curl_commands(&self) -> Result<(), anyhow::Error> {
        let mut reader = BufReader::new(io::stdin());
        let mut line = String::new();

        println!("Enter curl commands, type 'done' to exit:");

        while reader.read_line(&mut line).await? != 0 {
            let command = line.trim();
            if command == "done" {
                break;
            }

            let output = Command::new("sh").arg("-c").arg(command).output().await?;

            if output.status.success() {
                let response = String::from_utf8_lossy(&output.stdout);
                println!("Response: {}", response);
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);
                println!("Error: {}", error_message);
            }

            line.clear(); // Clear the line buffer for the next input
        }

        Ok(())
    }

    pub async fn install_flakebox_files(&self, repo_dir: &PathBuf) -> Result<(), anyhow::Error> {
        info!("Installing flakebox files...");

        // List of directories to copy from this level into the repo dir
        let directories_to_copy = vec![".config", ".github", "misc"];
        for dir in directories_to_copy {
            let source = PathBuf::from(format!("{}", dir));
            let destination = repo_dir;
            tokio::fs::create_dir_all(&destination).await?;
            // Assuming recursive copy is needed
            let mut options = CopyOptions::new(); // Initialize default options
            options.copy_inside = true; // To copy the contents into the destination
            dir::copy(&source, &destination, &options)?;
        }

        // List of files to copy
        let files_to_copy = vec!["justfile"];
        for file in files_to_copy {
            let source = PathBuf::from(format!("{}", file));
            let destination = repo_dir.join(file);
            // create the file if it doesn't exist
            if !destination.exists() {
                tokio::fs::File::create(&destination).await?;
            }
            tokio::fs::copy(&source, &destination).await?;
        }

        Ok(())
    }
}
