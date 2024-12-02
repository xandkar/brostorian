use std::{collections::HashMap, iter, path::Path, sync::LazyLock, time::Duration};

use futures::{Stream, StreamExt};
use url::Url;

const BAR_WIDTH: usize = 80; // TODO Config.
static BAR: LazyLock<String> = LazyLock::new(|| iter::repeat('-').take(BAR_WIDTH).collect());

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

pub async fn explore(hist_db_file: &Path, top_n: usize) -> anyhow::Result<()> {
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
    let mut local_paths: HashMap<String, u64> = HashMap::new();
    let mut domains: HashMap<String, u64> = HashMap::new();
    while let Some(row_result) = stream.next().await {
        let row = row_result?;
        let visits = u64::try_from(row.visit_count)?;
        let url = Url::parse(&row.url)?;
        match url.domain().map(|d| d.to_string()) {
            None => {
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
    domains.sort_by(|(_, a_count), (_, b_count)| b_count.cmp(a_count));
    let mut local_paths: Vec<(String, u64)> = local_paths.into_iter().collect();
    local_paths.sort_by(|(a_path, a_count), (b_path, b_count)| {
        // Order by count, then by path.
        if a_count == b_count {
            b_path.cmp(a_path)
        } else {
            b_count.cmp(a_count)
        }
    });

    print_counts("domains", domains.into_iter(), top_n);
    print_counts("local paths", local_paths.into_iter(), top_n);

    Ok(())
}

fn print_counts(name: &str, counts: impl Iterator<Item = (String, u64)>, top_n: usize) {
    println!("Top {top_n} {name}:");
    println!("{}", *BAR);
    for (rank, (name, count)) in counts.take(top_n).enumerate() {
        let rank = rank + 1;
        println!("{rank:3} {count:6} {name}");
    }
    println!();
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
