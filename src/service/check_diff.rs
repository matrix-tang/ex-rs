use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use binance::ws_model::WebsocketEvent;
use rust_decimal::Decimal;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use binance::api::*;
use binance::general::General;
use binance::websockets::*;
use chrono::Local;
use redis::{AsyncCommands, RedisResult};
use tracing::{error, info, warn};
use crate::{conf, db};
use serde::{Deserialize, Serialize};

const THREADS: i64 = 10;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PriceInfo {
    pub base_asset: String,
    pub quote_asset: String,
    pub price: Decimal,
    pub updated: u64,
}

// pub type Symbols = Arc<Mutex<DashMap<String, PriceInfo>>>;

#[derive(Debug, Clone)]
pub struct CheckDiff {
    pub senders: HashMap<i64, UnboundedSender<WebsocketEvent>>,
}

impl CheckDiff {
    pub fn new() -> Self {
        let mut txs = HashMap::new();
        for i in 0..THREADS {
            let (tx, mut rx) = mpsc::unbounded_channel::<WebsocketEvent>();
            txs.insert(i, tx.clone());

            tokio::spawn(async move {
                let cache = db::get_async_coin_symbols_cache().unwrap();
                loop {
                    select! {
                        event = rx.recv() => {
                            // println!("{:?}", event);
                            if let Some(WebsocketEvent::DayTicker(tick_event)) = event {
                                if let Ok(option_price_info) = cache.get_symbols(&tick_event.symbol) {
                                    if let Some(price_info) = option_price_info {
                                        let key = format!("{}{}", conf::vars::EX_PREFIX, &price_info.base_asset);
                                        if let Ok(option_symbols) = cache.get_coin_symbols(&key) {
                                            if let Some(symbols) = option_symbols {
                                                // println!("-----xxxx------xxxxx: {:?}, {:?}", cache.symbols.len(), cache.coin_symbols.len());
                                                info!("{:?}, {:?}, {:?}", tick_event.symbol, price_info, symbols);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        CheckDiff {
            senders: txs,
        }
    }

    #[allow(dead_code)]
    pub async fn init_coin_symbols(&self) -> anyhow::Result<()> {
        let client: General = Binance::new(None, None);
        let mut redis = db::get_redis_connection().await?;
        let cache = db::database::get_async_coin_symbols_cache().unwrap();
        if let Ok(exchange_info) = client.exchange_info().await {
            for symbol in exchange_info.symbols {
                let key = format!("{}{}", conf::vars::EX_PREFIX, &symbol.base_asset);
                let _: RedisResult<bool> = redis.hset(&key, &symbol.symbol, "1".to_string()).await;
                let _ = cache.set_coin_symbols(key, symbol.symbol);
            }
        }
        warn!("init coin symbols {:?}", Local::now().timestamp_millis());

        tokio::spawn(async move {
            loop {
                select! {
                    _oks = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                        if let Ok(exchange_info) = client.exchange_info().await {
                            for symbol in exchange_info.symbols {
                                let key = format!("{}{}", conf::vars::EX_PREFIX, &symbol.base_asset);
                                if let Ok(option_coin_symbols) = cache.get_coin_symbols(&key) {
                                    if let Some(coin_symbols) = option_coin_symbols {
                                        if coin_symbols.len() > 0 {
                                            continue
                                        }
                                    }
                                }
                                let _ = cache.set_coin_symbols(&key, symbol.symbol);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn init_symbols(&self) -> anyhow::Result<()> {
        let client: General = Binance::new(None, None);
        let cache = db::database::get_async_coin_symbols_cache().unwrap();
        if let Ok(exchange_info) = client.exchange_info().await {
            for symbol in exchange_info.symbols {
                cache.set_symbols(&symbol.symbol, PriceInfo {
                    base_asset: symbol.base_asset,
                    quote_asset: symbol.quote_asset,
                    price: Decimal::ZERO,
                    updated: 0,
                })?;
            }
        }
        warn!("init symbols {:?}", Local::now().timestamp_millis());

        tokio::spawn(async move {
            loop {
                select! {
                    _oks = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => {
                        if let Ok(exchange_info) = client.exchange_info().await {
                            for symbol in exchange_info.symbols {

                                if let Ok(_info) = cache.get_symbols(&symbol.symbol) {
                                    continue;
                                }

                                if let Err(e) = cache.set_symbols(&symbol.symbol, PriceInfo {
                                    base_asset: symbol.base_asset,
                                    quote_asset: symbol.quote_asset,
                                    price: Decimal::ZERO,
                                    updated: 0,
                                }) {
                                    error!("insert symbols error: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn last_price(&self, close_tx: UnboundedSender<bool>) -> anyhow::Result<()> {
        let txs = self.senders.clone();

        tokio::spawn(async move {
            let keep_running = AtomicBool::new(true);
            let all_ticker = all_ticker_stream();

            let cache = db::get_async_coin_symbols_cache().unwrap();

            let mut web_socket: WebSockets<'_, Vec<WebsocketEvent>> = WebSockets::new(|events: Vec<WebsocketEvent>| {
                for tick_events in events {
                    if let WebsocketEvent::DayTicker(tick_event) = tick_events.clone() {
                        // println!("{:?}", tick_event.clone());
                        let sharding = &tick_event.last_trade_id % THREADS;
                        if let Some(tx) = txs.get(&sharding) {
                            if let Err(e) = tx.send(tick_events.clone()) {
                                error!("send tick events to channel error: {:?}", e);
                                break;
                            }
                        }

                        if let Ok(option_price_info) = cache.get_symbols(&tick_event.symbol) {
                            if let Some(price_info) = option_price_info {
                                if let Err(e) = cache.set_symbols(&tick_event.symbol, PriceInfo {
                                    base_asset: price_info.base_asset,
                                    quote_asset: price_info.quote_asset,
                                    price: Decimal::from_str(tick_event.current_close.as_str()).unwrap_or_default(),
                                    updated: tick_event.event_time,
                                }) {
                                    error!("last price insert symbols error: {:?}", e);
                                }
                            }
                        }
                    }
                }

                Ok(())
            });

            if let Err(e) = web_socket.connect(all_ticker).await {
                error!("connect websocket error: {:?}", e);
                close_tx.send(true).unwrap();
            }
            if let Err(e) = web_socket.event_loop(&keep_running).await {
                error!("websocket event loop error: {:?}", e);
                close_tx.send(true).unwrap();
            }
            if let Err(e) = web_socket.disconnect().await {
                error!("disconnect websocket error: {:?}", e);
            }
            warn!("disconnected");
        });

        Ok(())
    }
}