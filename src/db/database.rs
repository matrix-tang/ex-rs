use anyhow::anyhow;
use crate::conf::config::Conf;
use once_cell::sync::OnceCell;
use redis::Client;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

static M_POOL: OnceCell<MySqlPool> = OnceCell::new();
static R_REDIS: OnceCell<Client> = OnceCell::new();

pub async fn init_db() -> anyhow::Result<()> {
    let c = Conf::get();
    // set mysql pool
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&c.mysql.url)
        .await?;
    M_POOL.set(pool).unwrap();

    // set redis
    let client = Client::open(c.redis.url.as_str())?;
    R_REDIS.set(client).unwrap();

    Ok(())
}

pub fn get_async_mysql_pool<'a>() -> Option<&'a MySqlPool> {
    M_POOL.get()
}

pub fn get_async_redis<'a>() -> Option<&'a Client> {
    R_REDIS.get()
}

pub async fn get_redis_connection() -> anyhow::Result<redis::aio::MultiplexedConnection> {
    match R_REDIS.get() {
        Some(client) => {
            client
                .get_multiplexed_tokio_connection()
                .await
                .map_err(|e|e.into())
        },
        None => {
            Err(anyhow!("get redis connection error. empty"))
        }
    }
}

#[cfg(test)]
mod tests {
    use redis::{AsyncCommands, RedisResult};
    use super::*;

    #[tokio::test]
    async fn test_database() {
        let _ = init_db().await;
        let pool = get_async_mysql_pool().unwrap();
        let row: (i64, String, String, String, Option<i64>, Option<i64>, Option<String>) = sqlx::query_as("SELECT * from p_config")
        .fetch_one(pool)
        .await.unwrap();
        println!("{:#?}", row);

        let mut redis = get_redis_connection().await.unwrap();
        let s: RedisResult<String> = redis.set("name", "tang").await;
        println!("{:#?}", s);
        let r: RedisResult<String> = redis.get("name").await;
        println!("{:#?}", r);
    }
}