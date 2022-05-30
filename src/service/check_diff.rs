use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use binance::ws_model::WebsocketEvent;
use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedSender};
use binance::websockets::*;
use lazy_static::lazy_static;
use tracing::{error, info, warn};

const THREADS: i64 = 10;

#[derive(Clone, Debug, Default)]
pub struct PriceInfo {
    pub price: Decimal,
    pub updated: u64,
}

pub type Symbols = DashMap<String, PriceInfo>;

lazy_static! {
    pub static ref SYMBOLS: Symbols = Symbols::default();
}

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
                loop {
                    select! {
                        event = rx.recv() => {
                            if let Some(WebsocketEvent::DayTicker(tick_event)) = event {
                                // info!("{:?}", tick_event);
                                if let Some(info) = SYMBOLS.get(&tick_event.symbol) {
                                    info!("{:?}, {:?}", info.price, info.updated);
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

    pub async fn last_price(&self) {
        let txs = self.senders.clone();
        let keep_running = AtomicBool::new(true);
        let all_ticker = all_ticker_stream();

        let mut web_socket: WebSockets<'_, Vec<WebsocketEvent>> = WebSockets::new(|events: Vec<WebsocketEvent>| {
            for tick_events in events {
                if let WebsocketEvent::DayTicker(tick_event) = tick_events.clone() {
                    let sharding = &tick_event.last_trade_id % THREADS;
                    if let Some(tx) = txs.get(&sharding) {
                        if let Err(e) = tx.send(tick_events.clone()) {
                            error!("{:?}", e);
                            continue;
                        }
                    }

                    SYMBOLS.insert(tick_event.symbol.clone(), PriceInfo {
                        price: Decimal::from_str(tick_event.current_close.as_str()).unwrap(),
                        updated: tick_event.event_time,
                    });
                }
            }

            Ok(())
        });

        web_socket.connect(all_ticker).await.unwrap(); // check_diff error
        if let Err(e) = web_socket.event_loop(&keep_running).await {
            error!("Error: {:?}", e);
        }
        web_socket.disconnect().await.unwrap();
        warn!("disconnected");
    }
}