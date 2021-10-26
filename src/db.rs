use anyhow::{Context, Result};
use diesel::{
    pg::PgConnection,
    r2d2,
    r2d2::{ConnectionManager, PooledConnection},
};
use dotenv::dotenv;
use std::env;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool() -> Result<Pool> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool = Pool::new(manager).context("Failed to create pool object")?;

    Ok(pool)
}
