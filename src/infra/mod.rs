use anyhow::Result;
use sqlx::SqlitePool;

use crate::infra::sqlite::schema::INIT_SQL;

pub mod crypto;
pub mod sqlite;

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::query(INIT_SQL).execute(&pool).await?;
    Ok(pool)
}
