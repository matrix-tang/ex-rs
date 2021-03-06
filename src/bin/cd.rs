use tokio::select;
use tracing::{info, Level, warn};
use ex_rs::db;
use ex_rs::service::check_diff;
use time::{macros::format_description, UtcOffset};
use tracing_subscriber::{fmt::time::OffsetTime, EnvFilter};
use ex_rs::conf::config::Conf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::get();
    let log_path = env!("CARGO_MANIFEST_DIR").to_string() + &conf.log.path;
    let log_name = &conf.log.name;

    // let file_appender = tracing_appender::rolling::hourly(log_path, log_name);
    // 日志输出到文件
    let file_appender = tracing_appender::rolling::never(log_path, log_name);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 设置日志日期格式
    let local_time = OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    );

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(local_time)
        .with_max_level(Level::INFO)
        // sets this to be the default, global collector for this application.
        .init();
    warn!("check diff ...");

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
    c.book_ticker(close_tx.clone()).await?;

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