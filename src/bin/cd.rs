use tracing::info;
use ex_rs::service::check_diff::CheckDiff;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("--------------- cd");

    let c = CheckDiff::new();
    c.last_price().await;
}