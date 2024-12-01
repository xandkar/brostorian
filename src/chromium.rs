use std::{collections::HashMap, path::Path, time::Duration};

use futures::{Stream, StreamExt};
use url::Url;

// CREATE TABLE urls(
//      id INTEGER PRIMARY KEY AUTOINCREMENT,
//      url LONGVARCHAR,
//      title LONGVARCHAR,
//      visit_count INTEGER DEFAULT 0 NOT NULL,
//      typed_count INTEGER DEFAULT 0 NOT NULL,
//      last_visit_time INTEGER NOT NULL,
//      hidden INTEGER DEFAULT 0 NOT NULL
// );

#[derive(sqlx::FromRow, Debug)]
struct UrlRow {
    id: i64,
    url: String,
    title: String,
    visit_count: i64,
    typed_count: i64,
    last_visit_time: i64,
    hidden: i64,
}

pub async fn explore(hist_db_file: &Path) -> anyhow::Result<()> {
    let busy_timeout = Duration::from_secs(60); // TODO Config.
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(hist_db_file)
        .read_only(true)
        .create_if_missing(false)
        .busy_timeout(busy_timeout);
    let pool = sqlx::SqlitePool::connect_with(options).await?;
    println!("count={:?}", count(&pool).await?);
    println!("head={:#?}", head(&pool, 2).await?);
    let mut stream = stream(&pool);
    let mut local_paths = HashMap::new();
    let mut domains: HashMap<String, u64> = HashMap::new();
    while let Some(row_result) = stream.next().await {
        let row = row_result?;
        let url = Url::parse(&row.url)?;
        let visits = u64::try_from(row.visit_count)?;
        match url.domain().map(|d| d.to_string()) {
            None => {
                tracing::warn!(?row, "Domain could not be parsed.");
                let path = url.path().to_string();
                local_paths
                    .entry(path)
                    .and_modify(|count| *count += visits)
                    .or_insert(visits);
            }
            Some(domain) => {
                domains
                    .entry(domain)
                    .and_modify(|count| *count += visits)
                    .or_insert(visits);
            }
        }
    }
    let mut domains: Vec<(String, u64)> = domains.into_iter().collect();
    domains.sort_by_key(|(_, count)| *count);
    let mut local_paths: Vec<(String, u64)> = local_paths.into_iter().collect();
    local_paths.sort_by(|(a_path, a_count), (b_path, b_count)| {
        // Order by count, then by path.
        if a_count == b_count {
            b_path.cmp(a_path)
        } else {
            a_count.cmp(b_count)
        }
    });
    for (domain, count) in domains {
        println!("{count} {domain}");
    }
    println!();
    for (path, count) in local_paths {
        println!("{count} {path}");
    }
    Ok(())
}

async fn count(pool: &sqlx::Pool<sqlx::Sqlite>) -> anyhow::Result<u64> {
    let (count,): (u64,) = sqlx::query_as("SELECT COUNT(*) FROM urls")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

async fn head(pool: &sqlx::Pool<sqlx::Sqlite>, limit: i64) -> anyhow::Result<Vec<UrlRow>> {
    let rows: Vec<UrlRow> = sqlx::query_as("SELECT * FROM urls LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

fn stream<'a>(pool: &'a sqlx::Pool<sqlx::Sqlite>) -> impl Stream<Item = sqlx::Result<UrlRow>> + 'a {
    sqlx::query_as("SELECT * FROM urls").fetch(pool)
}
