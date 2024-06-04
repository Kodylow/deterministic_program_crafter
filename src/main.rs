use clap::Parser;
use git2::Repository;
use reqwest::Client;
use serde::Deserialize;

pub mod config;

#[derive(Deserialize)]
struct Crate {
    crate_id: String,
    repository: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli_args = config::CliArgs::parse();

    let client = Client::new();
    let request_url = format!("https://crates.io/api/v1/crates/{}", cli_args.tool);
    let response = client.get(&request_url).send().await.unwrap();

    if response.status().is_success() {
        let crate_data: Crate = response.json().await.unwrap();
        if let Some(repo_url) = crate_data.repository {
            println!("Cloning repository: {}", repo_url);
            match Repository::clone(&repo_url, &cli_args.work_dir) {
                Ok(_) => println!("Repository cloned successfully."),
                Err(e) => println!("Failed to clone repository: {}", e),
            }
        } else {
            println!("No repository found for {}", cli_args.tool);
        }
    } else {
        println!("Failed to fetch data for {}", cli_args.tool);
    }
}
