use anyhow::anyhow;
use crate::conf::config::Conf;
use once_cell::sync::OnceCell;
use redis::Client;
use sled::Db;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use crate::helpers::coin_symbol::CoinSymbolCache;

static M_POOL: OnceCell<MySqlPool> = OnceCell::new();
static R_REDIS: OnceCell<Client> = OnceCell::new();
static SLED_DB: OnceCell<Db> = OnceCell::new();
static COIN_SYMBOLS: OnceCell<CoinSymbolCache> = OnceCell::new();

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

    let sled_db = sled::open(&c.sled.path.as_str())?;
    SLED_DB.set(sled_db).unwrap();

    COIN_SYMBOLS.set(CoinSymbolCache::new()).unwrap();

    Ok(())
}

pub fn get_async_mysql_pool<'a>() -> Option<&'a MySqlPool> {
    M_POOL.get()
}

pub fn get_async_redis<'a>() -> Option<&'a Client> {
    R_REDIS.get()
}

pub fn get_async_sled_db<'a>() -> Option<&'a Db> {
    SLED_DB.get()
}

pub fn get_async_coin_symbols_cache<'a>() -> Option<&'a CoinSymbolCache> {
    COIN_SYMBOLS.get()
}

pub async fn get_redis_connection() -> anyhow::Result<redis::aio::MultiplexedConnection> {
    match R_REDIS.get() {
        Some(client) => {
            client
                .get_multiplexed_tokio_connection()
                .await
                .map_err(|e| e.into())
        }
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

        let sled_db = get_async_sled_db().unwrap();
        let _ = sled_db.insert("name", "tang1");
        let name = sled_db.get("name");
        println!("{:#?}", name);
    }
}