use dashmap::DashMap;
use crate::service::check_diff::PriceInfo;

#[derive(Debug, Clone)]
pub struct CoinSymbolCache {
    pub coin_symbols: DashMap<String, Vec<String>>,
    pub symbols: DashMap<String, PriceInfo>,
}

impl CoinSymbolCache {
    pub fn new() -> Self {
        Self {
            coin_symbols: Default::default(),
            symbols: Default::default(),
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