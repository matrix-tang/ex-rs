use tokio::select;
use tracing::info;
use ex_rs::db;
use ex_rs::service::check_diff;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("check diff ...");

    let (close_tx, mut close_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
    let wait_loop = tokio::spawn(async move {
        'hello: loop {
            select! {
                _ = close_rx.recv() => break 'hello
            }
        }
    });

    db::init_db().await?;
    let c = check_diff::CheckDiff::new();
    c.init_coin_symbols().await?;
    c.init_symbols().await?;
    c.last_price(close_tx.clone()).await?;

    select! {
        _ = wait_loop => {
            info!("Finished!");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Closing websocket stream...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    Ok(())
}