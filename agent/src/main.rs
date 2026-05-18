use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tradeautopsy_agent=debug,info".into()),
        )
        .init();

    let config = tradeautopsy_agent::AgentConfig::from_env()?;
    tradeautopsy_agent::run_agent(config).await
}
