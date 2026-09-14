use sqlx::SqlitePool;
use tracing::info;

pub async fn init_db() -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect("sqlite://tasks.db").await?;
    sqlx::migrate!().run(&pool).await?;
    info!("Database initialized");
    Ok(pool)
}
