use trovr::{build_state, health_check, init_tracing, run_migrations, shutdown_signal};
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

    let state = build_state(&config, pool)?;
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(listener, trovr_api::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
