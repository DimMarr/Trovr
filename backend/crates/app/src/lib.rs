use tracing_subscriber::EnvFilter;
use trovr_config::LogFormat;

pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

pub async fn health_check(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("SELECT 1 AS one").fetch_one(pool).await?;
    Ok(())
}

pub fn init_tracing(format: &LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}
