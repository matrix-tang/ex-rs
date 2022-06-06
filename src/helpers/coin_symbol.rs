use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PriceInfo {
    pub base_asset: String,
    pub quote_asset: String,
    pub price: Decimal,
    pub updated: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BookTicker {
    pub update_id: u64,
    pub symbol: String,
    pub best_bid: Decimal,
    pub best_bid_qty: Decimal,
    pub best_ask: Decimal,
    pub best_ask_qty: Decimal,
}

#[derive(Debug, Clone)]
pub struct CoinSymbolCache {
    pub coin_symbols: DashMap<String, Vec<String>>,
    pub symbols: DashMap<String, PriceInfo>,
    pub book_tickers: DashMap<String, BookTicker>,
}

impl CoinSymbolCache {
    pub fn new() -> Self {
        Self {
            coin_symbols: Default::default(),
            symbols: Default::default(),
            book_tickers: Default::default(),
        }
    }

    pub fn set_coin_symbols<C>(&self, coin: C, symbol: String) -> anyhow::Result<()>
        where
            C: Into<String>
    {
        self.coin_symbols
            .entry(coin.into())
            .and_modify(|symbols| {
                symbols.push(symbol.clone());
            })
            .or_insert_with(|| {
                let mut v = Vec::new();
                v.push(symbol.clone());
                v
            });
        Ok(())
    }

    pub fn get_coin_symbols<C>(&self, coin: C) -> anyhow::Result<Option<Vec<String>>>
        where
            C: Into<String>
    {
        Ok(self.coin_symbols.get(&coin.into()).map_or(Option::from(vec![]), |v| Option::from(v.value().clone())))
    }

    pub fn set_book_ticker<S>(&self, symbol: S, book_ticker: BookTicker) -> anyhow::Result<()>
        where
            S: Into<String>,
    {
        self.book_tickers.insert(symbol.into(), book_ticker);
        Ok(())
    }

    pub fn get_book_ticker<S>(&self, symbol: S) -> anyhow::Result<Option<BookTicker>>
        where
            S: Into<String>,
    {
        Ok(self.book_tickers.get(&symbol.into()).map_or(Option::from(BookTicker {
            update_id: 0,
            symbol: "".to_string(),
            best_bid: Default::default(),
            best_bid_qty: Default::default(),
            best_ask: Default::default(),
            best_ask_qty: Default::default(),
        }), |v| Option::from(v.value().clone())))
    }

    pub fn set_symbols<S>(&self, symbol: S, price_info: PriceInfo) -> anyhow::Result<()>
        where
            S: Into<String>
    {
        self.symbols.insert(symbol.into(), price_info);
        Ok(())
    }

    pub fn get_symbols<S>(&self, symbol: S) -> anyhow::Result<Option<PriceInfo>>
        where
            S: Into<String>
    {
        Ok(self.symbols.get(&symbol.into()).map_or(Option::from(PriceInfo::default()), |v| Option::from(v.value().clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coin_symbols_cache() {
        println!("----- test_coin_symbols cache");
        let cache = CoinSymbolCache::new();
        let result = cache.set_coin_symbols("BTC".to_string(), "BTCUDST".to_string());
        println!("coin symbols result: {:?}", result);
        let result = cache.set_coin_symbols("BTC".to_string(), "BTCBUSD".to_string());
        println!("coin symbols result: {:?}", result);
        let result = cache.get_coin_symbols("BTC".to_string());
        println!("coin symbols result: {:?}", result);

        let result = cache.set_symbols("BTCUSDT".to_string(), PriceInfo {
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            price: Default::default(),
            updated: 0,
        });
        println!("symbols result: {:?}", result);
        let result = cache.get_symbols("BTCUSDT".to_string());
        println!("symbols result: {:?}", result);
    }
}