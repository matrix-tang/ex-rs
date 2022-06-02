use tokio::select;
use tracing::info;
use ex_rs::db;
use ex_rs::service::check_diff::{CheckDiff};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("--------------- cd");

    db::init_db().await?;
    let c = CheckDiff::new();
    c.init_coin_symbols().await?;
    c.init_symbols().await?;
    c.last_price().await?;

    select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Closing websocket stream...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    Ok(())
}