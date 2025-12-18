use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::env;

use website::services::activity_geo_service;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL moet in .env staan");
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Kan niet verbinden met DB");

    let limit: i64 = env::var("BACKFILL_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);

    match activity_geo_service::backfill_activity_geo(&pool, limit).await {
        Ok(report) => {
            println!(
                "geo backfill: candidates={}, updated={}, skipped={}, failed={}",
                report.candidates, report.updated, report.skipped, report.failed
            );
        }
        Err(e) => {
            eprintln!("geo backfill failed: {}", e);
            std::process::exit(1);
        }
    }
}
