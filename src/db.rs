use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

pub async fn init_pool() -> Result<PgPool> {
    let db_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .context(format!("Could not connect to db, tried {}", db_url))?;

    Ok(pool)
}
