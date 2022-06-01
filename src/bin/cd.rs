use tokio::select;
use tracing::info;
use ex_rs::db;
use ex_rs::db::init_db;
use ex_rs::service::check_diff::{CheckDiff};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("--------------- cd");

    init_db().await?;

    // let sled_db = sled::open(&Conf::get().sled.path)?;

    let sled_db = db::get_async_sled_db().unwrap();

    let c = CheckDiff::new(&sled_db);
    c.init_coin_symbols().await?;
    c.init_symbols(&sled_db).await?;
    c.last_price(&sled_db).await?;

    select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Closing websocket stream...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    Ok(())
}