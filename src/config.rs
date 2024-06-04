use clap::Parser;

/// Deterministic Program Crafter is an agent tool for building other agent
/// tools correctly, deterministically, and reproducibly.
#[derive(Parser)]
#[clap(version = "1.0", author = "Kody Low")]
pub struct CliArgs {
    /// The agent instructions
    #[clap(short, long)]
    pub instructions: String,

    /// The directory to clone the repository into
    #[clap(short, long, default_value = "./work_dir")]
    pub work_dir: std::path::PathBuf,

    /// The Groq API key
    #[clap(short, long, env = "GROQ_API_KEY")]
    pub groq_api_key: String,

    /// Cookie for crates.io session
    #[clap(short, long, env = "CARGO_COOKIE")]
    pub cargo_cookie: String,
}
