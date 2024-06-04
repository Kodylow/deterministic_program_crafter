use app::App;
use clap::Parser;

pub mod app;
pub mod config;
pub mod groq;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logging_and_env()?;

    let cli_args = config::CliArgs::parse();
    let app = App::new(&cli_args).await;

    app.run().await
}

fn init_logging_and_env() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    Ok(())
}
