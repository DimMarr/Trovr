use trovr::{health_check, init_tracing, run_migrations};
use trovr_config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    init_tracing(&config.log_format);

    tracing::info!(bind_addr = %config.bind_addr, "starting trovr");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    run_migrations(&pool).await?;
    health_check(&pool).await?;

    tracing::info!("migrations applied and database reachable");

    Ok(())
}
