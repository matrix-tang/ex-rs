use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use anyhow::anyhow;
use binance::ws_model::WebsocketEvent;
use rust_decimal::Decimal;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use binance::api::*;
use binance::general::General;
use binance::websockets::*;
use redis::{AsyncCommands, RedisResult};
use tracing::{error, info, warn};
use crate::{conf, db};
use serde::{Deserialize, Serialize};
use sled::IVec;

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

            let sled_db = db::get_async_sled_db().unwrap();

            tokio::spawn(async move {
                let mut redis = db::get_redis_connection().await.unwrap();
                loop {
                    select! {
                        event = rx.recv() => {
                            // println!("{:?}", event);
                            if let Some(WebsocketEvent::DayTicker(tick_event)) = event {
                                if let Ok(symbol) = sled_db.get(&tick_event.symbol) {
                                    if let Some(iv) = symbol {
                                        if let Ok(t) = serde_json::from_slice(iv.as_ref()) {
                                            let info: PriceInfo = t;
                                            let key = format!("{}{}", conf::vars::EX_PREFIX, &info.base_asset);
                                            let r: RedisResult<HashMap<String, String>> = redis.hgetall(key).await;
                                            let _ = r.map(|x| {
                                                // println!("xxxxxxxxx: {:?}", x.into_keys());
                                                info!("{:?}, {:?}, {:?}", tick_event.symbol, info, x.into_keys());
                                            });
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
        if let Ok(exchange_info) = client.exchange_info().await {
            for symbol in exchange_info.symbols {
                let key = format!("{}{}", conf::vars::EX_PREFIX, &symbol.base_asset);
                let _: RedisResult<bool> = redis.hset(&key, &symbol.symbol, "1".to_string()).await;
                /*let r: RedisResult<HashMap<String, String>> = redis.hgetall(key).await;
                r.map(|x| {
                    println!("xxxxxxxxx: {:?}", x.into_keys());
                });*/
            }
        }

        tokio::spawn(async move {
            loop {
                select! {
                    _oks = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                        if let Ok(exchange_info) = client.exchange_info().await {
                            for symbol in exchange_info.symbols {
                                let key = format!("{}{}", conf::vars::EX_PREFIX, &symbol.base_asset);
                                // println!("---------------------{:?}", key);
                                let result: RedisResult<bool> = redis.hget(&key, &symbol.symbol).await;
                                if let Ok(ex) = result {
                                    if ex {
                                        continue;
                                    }
                                }

                                let _: RedisResult<bool> = redis.hset(&key, &symbol.symbol, "1".to_string()).await;
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
        let sled_db = db::get_async_sled_db().unwrap();
        if let Ok(exchange_info) = client.exchange_info().await {
            for symbol in exchange_info.symbols {
                let s = serde_json::to_string(&PriceInfo {
                    base_asset: symbol.base_asset,
                    quote_asset: symbol.quote_asset,
                    price: Decimal::ZERO,
                    updated: 0,
                })?;
                sled_db.insert(&symbol.symbol, IVec::from(s.as_str()))?;
            }
        }

        tokio::spawn(async move {
            loop {
                select! {
                    _oks = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                        if let Ok(exchange_info) = client.exchange_info().await {
                            for symbol in exchange_info.symbols {
                                if let Ok(_symbol) = sled_db.get(&symbol.symbol) {
                                    continue;
                                }

                                match serde_json::to_string(&PriceInfo {
                                    base_asset: symbol.base_asset,
                                    quote_asset: symbol.quote_asset,
                                    price: Decimal::ZERO,
                                    updated: 0,
                                }) {
                                    Ok(s) => {
                                        if let Err(e) = sled_db.insert(symbol.symbol, IVec::from(s.as_str())) {
                                            error!("insert symbols error: {:?}", e);
                                        }
                                    },
                                    Err(e) => {
                                        error!("serde json error: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn last_price(&self) -> anyhow::Result<()> {
        let txs = self.senders.clone();

        tokio::spawn(async move {
            let keep_running = AtomicBool::new(true);
            let all_ticker = all_ticker_stream();

            let sled = db::get_async_sled_db().unwrap();

            let mut web_socket: WebSockets<'_, Vec<WebsocketEvent>> = WebSockets::new(|events: Vec<WebsocketEvent>| {
                for tick_events in events {
                    if let WebsocketEvent::DayTicker(tick_event) = tick_events.clone() {
                        // println!("{:?}", tick_event.clone());
                        let sharding = &tick_event.last_trade_id % THREADS;
                        if let Some(tx) = txs.get(&sharding) {
                            if let Err(e) = tx.send(tick_events.clone()) {
                                error!("{:?}", e);
                                break;
                            }
                        }

                        if let Ok(symbol) = sled.get(&tick_event.symbol) {
                            if let Some(iv) = symbol {
                                if let Ok(t) = serde_json::from_slice(iv.as_ref()) {
                                    let info: PriceInfo = t;
                                    match serde_json::to_string(&PriceInfo {
                                        base_asset: info.base_asset,
                                        quote_asset: info.quote_asset,
                                        price: Decimal::from_str(tick_event.current_close.as_str()).unwrap(),
                                        updated: tick_event.event_time,
                                    }) {
                                        Ok(s) => {
                                            if let Err(e) = sled.insert(tick_event.symbol, IVec::from(s.as_str())) {
                                                error!("last price insert symbols error: {:?}", e);
                                            }
                                        }
                                        Err(e) => {
                                            error!("serde json error: {:?}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(())
            });

            web_socket.connect(all_ticker).await.unwrap(); // check_diff error
            if let Err(e) = web_socket.event_loop(&keep_running).await {
                error!("Error: {:?}", e);
                return Err(anyhow!(e));
            }
            web_socket.disconnect().await.unwrap();
            warn!("disconnected");

            Ok(())
        });

        Ok(())
    }
}