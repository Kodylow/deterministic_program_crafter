use clap::Parser;

/// Deterministic Program Crafter is an agent tool for building other agent
/// tools correctly, deterministically, and reproducibly.
#[derive(Parser)]
#[clap(version = "1.0", author = "Kody Low")]
pub struct CliArgs {
    /// The agent instructions
    #[clap(long)]
    pub instructions: String,

    /// The directory to clone the repository into
    #[clap(long, default_value = "./work_dir")]
    pub work_dir: std::path::PathBuf,

    /// The Groq API key
    #[clap(long, env = "GROQ_API_KEY")]
    pub groq_api_key: String,

    /// Github token for forking repositories
    #[clap(long, env = "GITHUB_TOKEN")]
    pub github_token: String,

    /// Cookie for crates.io session
    #[clap(long, env = "CARGO_COOKIE")]
    pub cargo_cookie: String,
}
