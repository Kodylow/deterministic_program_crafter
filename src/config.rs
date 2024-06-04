use clap::Parser;

/// Deterministic Program Crafter is an agent tool for building other agent tools correctly, deterministically, and reproducibly.
#[derive(Parser, Debug)]
#[command(
    version = "0.1.0",
    author = "Your Name",
    about = "Deterministic Program Crafter is an agent tool for building other agent tools correctly, deterministically, and reproducibly."
)]
pub struct CliArgs {
    /// The agent instructions
    #[arg(short, long)]
    pub instructions: String,

    /// The directory to clone the repository into
    #[arg(short, long, default_value = "./work_dir")]
    pub work_dir: std::path::PathBuf,
}
