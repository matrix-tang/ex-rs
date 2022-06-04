use dashmap::DashMap;
use crate::service::check_diff::PriceInfo;

#[derive(Debug, Clone)]
pub struct CoinSymbolCache {
    coin_symbols: DashMap<String, Vec<String>>,
    symbols: DashMap<String, PriceInfo>,
}

impl CoinSymbolCache {
    pub fn new() -> Self {
        Self {
            coin_symbols: Default::default(),
            symbols: Default::default(),
        }
    }

    pub fn set_coin_symbols(&self, coin: String, symbol: String) -> anyhow::Result<()> {
        self.coin_symbols
            .entry(coin)
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

    pub fn get_coin_symbols(&self, coin: String) -> anyhow::Result<Option<Vec<String>>> {
        Ok(self.coin_symbols.get(&coin).map(|v| v.value().clone()))
    }

    pub fn set_symbols(&self, symbol: String, price_info: PriceInfo) -> anyhow::Result<()> {
        self.symbols.insert(symbol, price_info);
        Ok(())
    }

    pub fn get_symbols(&self, symbol: String) -> anyhow::Result<Option<PriceInfo>> {
        Ok(self.symbols.get(&symbol).map(|s| s.value().clone()))
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