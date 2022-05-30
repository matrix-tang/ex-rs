// redis key
pub const REDIS_SPOT_PRICE_KEY: &str = "_spot_price";
pub const REDIS_FUTURES_PRICE_KEY: &str = "_futures_price";
pub const REDIS_DELIVERY_PRICE_KEY: &str = "_delivery_price";

// platform
pub const PLATFORM_BINANCE: &str = "binance";
pub const PLATFORM_HUOBI: &str = "huobi";
pub const PLATFORM_FTX: &str = "ftx";

// market
pub const SPOT: &str = "spot";
pub const FUTURES: &str = "futures";
pub const DELIVERY: &str = "delivery";

// trigger condition
pub const TRIGGER_GREAT_EQUAL: &str = "ge";
pub const TRIGGER_LESS_EQUAL: &str = "le";

// time in force
pub const TIME_IN_FORECE_GTC: &str = "GTC";
pub const TIME_IN_FORECE_IOC: &str = "IOC";
pub const TIME_IN_FORECE_FOK: &str = "FOK";

// order side
pub const ORDER_SIDE_BUY: &str = "buy";
pub const ORDER_SIDE_SELL: &str = "sell";
