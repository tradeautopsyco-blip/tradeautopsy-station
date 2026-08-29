use std::sync::Mutex;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tradeautopsy_agent=debug,info".into());

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log_dir = std::path::PathBuf::from(home).join("Library/Logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("tradeautopsy-agent.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let writer = std::io::stdout.and(Mutex::new(file));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .init();

    let config = tradeautopsy_agent::AgentConfig::from_env()?;
    tradeautopsy_agent::run_agent(config).await
}
