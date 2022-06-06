use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use binance::ws_model::{WebsocketEvent, WebsocketEventUntag};
use rust_decimal::Decimal;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use binance::api::*;
use binance::general::General;
use binance::websockets::*;
use chrono::Local;
use redis::{AsyncCommands, RedisResult};
use rust_decimal::prelude::FromPrimitive;
use tracing::{error, info, warn};
use crate::{conf, db};
use crate::helpers::coin_symbol::{PriceInfo, BookTicker};

const THREADS: i64 = 10;

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

    pub async fn book_ticker(&self, close_tx: UnboundedSender<bool>) -> anyhow::Result<()> {
        tokio::spawn(async move {
            let keep_running = AtomicBool::new(true);
            let all_book_ticker = all_book_ticker_stream();
            let cache = db::get_async_coin_symbols_cache().unwrap();
            let mut web_socket: WebSockets<'_, WebsocketEventUntag> = WebSockets::new(|events: WebsocketEventUntag| {
                if let WebsocketEventUntag::BookTicker(tick_event) = events {
                    let _ = cache.set_book_ticker(&tick_event.symbol, BookTicker {
                        update_id: tick_event.update_id,
                        symbol: tick_event.symbol.clone(),
                        best_bid: Decimal::from_f64(tick_event.best_bid).unwrap_or_default(),
                        best_bid_qty: Decimal::from_f64(tick_event.best_bid_qty).unwrap_or_default(),
                        best_ask: Decimal::from_f64(tick_event.best_ask).unwrap_or_default(),
                        best_ask_qty: Decimal::from_f64(tick_event.best_ask_qty).unwrap_or_default(),
                    });
                    /*let r = cache.get_book_ticker(&tick_event.symbol).unwrap();
                    println!("{:?}, {:?}", r, cache.book_tickers.len());*/
                }
                Ok(())
            });

            web_socket.connect(&all_book_ticker).await.unwrap(); // check error
            if let Err(e) = web_socket.event_loop(&keep_running).await {
                error!("book_ticker connect Error: {:?}", e);
                close_tx.send(true).unwrap();
            }
            web_socket.disconnect().await.unwrap();
            warn!("disconnected");
        });
        Ok(())
    }
}