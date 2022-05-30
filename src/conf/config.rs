use serde::Deserialize;

use std::fs::File;
use std::io::prelude::*;

use once_cell::sync::Lazy;

#[derive(Debug, Deserialize)]
pub struct IpConfig {
    pub name: String,
    pub ip: String,
    pub port: String,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct MysqlConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceApiConfig {
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub redis: RedisConfig,
    pub mysql: MysqlConfig,
    pub ip_config: Vec<IpConfig>,
    pub binance_api_config: BinanceApiConfig,
}

impl Conf {
    pub fn new() -> Conf {
        let config_path = env!("CARGO_MANIFEST_DIR");
        let path = config_path.to_string() + "/config.toml";

        let mut file = File::open(path).expect("no config file: for config.rs");
        let mut str_val = String::new();
        file.read_to_string(&mut str_val)
            .expect("read config file err: fro config.rs");
        toml::from_str(&str_val).expect("str to toml err: for config.rs")
    }

    pub fn get() -> &'static Conf {
        static INSTANCE: Lazy<Conf> = Lazy::new(|| self::Conf::new());
        &INSTANCE
    }

    pub fn get_binance_api_config() -> (&'static String, &'static String) {
        let conf = Conf::get();
        let api_key = &conf.binance_api_config.api_key;
        let secret_key = &conf.binance_api_config.secret_key;
        (api_key, secret_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config() {
        let c = Conf::get();
        println!("{:#?}", c.ip_config);
        println!("{:#?}", c.redis.url);
        println!("{:#?}", c.mysql.url);
        println!("{:#?}", c.binance_api_config);
    }
}
