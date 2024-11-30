use std::{path::Path, time::Duration};

pub async fn explore(hist_db_file: &Path) -> anyhow::Result<()> {
    let busy_timeout = Duration::from_secs(60); // TODO Config.
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(hist_db_file)
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(busy_timeout);
    let pool = sqlx::SqlitePool::connect_with(options).await?;
    println!("count={:?}", count(&pool).await?);
    Ok(())
}

async fn count(pool: &sqlx::Pool<sqlx::Sqlite>) -> anyhow::Result<u64> {
    let (count,): (u64,) = sqlx::query_as("SELECT COUNT(*) FROM urls")
        .fetch_one(pool)
        .await?;
    Ok(count)
}
