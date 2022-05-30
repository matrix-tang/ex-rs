pub mod database;

use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;
use crate::conf::config::Conf;

#[derive(Clone, Debug)]
pub struct Db {
    db_pool: MySqlPool,
    redis: redis::Client,
}

impl Db {
    pub async fn new() -> anyhow::Result<Self> {
        let c = Conf::get();
        let db_pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&c.mysql.url)
            .await?;
        let client = redis::Client::open(c.redis.url.as_str())?;
        Ok(Self {
            db_pool,
            redis: client
        })
    }

    pub fn database(&self) -> &MySqlPool {
        &self.db_pool
    }

    pub async fn redis(&self) -> anyhow::Result<redis::aio::MultiplexedConnection> {
        self.redis
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e|e.into())
    }
}

#[cfg(test)]
mod tests {
    use redis::{AsyncCommands, RedisResult};
    use super::*;

    #[tokio::test]
    async fn test_db() {
        let db = Db::new().await.unwrap();
        let row: (i64, String, String, String, Option<i64>, Option<i64>, Option<String>) = sqlx::query_as("SELECT * from p_config")
        .fetch_one(&db.db_pool)
        .await.unwrap();
        println!("{:#?}", row);

        let mut redis = db.redis().await.unwrap();
        let s: RedisResult<String> = redis.set("name", "tang").await;
        println!("{:#?}", s);
        let r: RedisResult<String> = redis.get("name").await;
        println!("{:#?}", r);
    }
}
