// Copyright (C) 2024 [Kulpreet Singh]
//
//  This file is part of P2Poolv2
//
// P2Poolv2 is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// P2Poolv2 is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// P2Poolv2. If not, see <https://www.gnu.org/licenses/>.

use bitcoin::PublicKey;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    pub listen_address: String,
    pub dial_peers: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StoreConfig {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CkPoolConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinerConfig {
    pub pubkey: PublicKey,
}

/// helper function to deserialize the network from the config file, which is provided as a string like Core
/// Possible values are: main, test, testnet4, signet, regtest
fn deserialize_network<'de, D>(deserializer: D) -> Result<bitcoin::Network, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    bitcoin::Network::from_core_arg(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct BitcoinConfig {
    #[serde(deserialize_with = "deserialize_network")]
    pub network: bitcoin::Network,
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub network: NetworkConfig,
    pub store: StoreConfig,
    pub ckpool: CkPoolConfig,
    pub miner: MinerConfig,
    pub bitcoin: BitcoinConfig,
}

#[allow(dead_code)]
impl Config {
    pub fn load(path: &str) -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?
            .try_deserialize()
    }

    pub fn with_listen_address(mut self, listen_address: String) -> Self {
        self.network.listen_address = listen_address;
        self
    }

    pub fn with_dial_peers(mut self, dial_peers: Vec<String>) -> Self {
        self.network.dial_peers = dial_peers;
        self
    }

    pub fn with_store_path(mut self, store_path: String) -> Self {
        self.store.path = store_path;
        self
    }

    pub fn with_ckpool_host(mut self, ckpool_host: String) -> Self {
        self.ckpool.host = ckpool_host;
        self
    }

    pub fn with_ckpool_port(mut self, ckpool_port: u16) -> Self {
        self.ckpool.port = ckpool_port;
        self
    }

    pub fn with_miner_pubkey(mut self, miner_pubkey: String) -> Self {
        self.miner.pubkey = miner_pubkey.parse().unwrap();
        self
    }

    pub fn with_bitcoin_url(mut self, bitcoin_url: String) -> Self {
        self.bitcoin.url = bitcoin_url;
        self
    }

    pub fn with_bitcoin_username(mut self, bitcoin_username: String) -> Self {
        self.bitcoin.username = bitcoin_username;
        self
    }

    pub fn with_bitcoin_password(mut self, bitcoin_password: String) -> Self {
        self.bitcoin.password = bitcoin_password;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = Config::load("./config.toml").unwrap();
        let config = config
            .with_listen_address("127.0.0.1:8080".to_string())
            .with_dial_peers(vec![
                "peer1.example.com".to_string(),
                "peer2.example.com".to_string(),
            ])
            .with_store_path("/tmp/store".to_string())
            .with_ckpool_host("ckpool.example.com".to_string())
            .with_ckpool_port(3333)
            .with_miner_pubkey(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string(),
            )
            .with_bitcoin_url("http://localhost:8332".to_string())
            .with_bitcoin_username("testuser".to_string())
            .with_bitcoin_password("testpass".to_string());

        assert_eq!(config.network.listen_address, "127.0.0.1:8080");
        assert_eq!(
            config.network.dial_peers,
            vec!["peer1.example.com", "peer2.example.com"]
        );
        assert_eq!(config.store.path, "/tmp/store");
        assert_eq!(config.ckpool.host, "ckpool.example.com");
        assert_eq!(config.ckpool.port, 3333);
        assert_eq!(
            config.miner.pubkey.to_string(),
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
        );
        assert_eq!(config.bitcoin.url, "http://localhost:8332");
        assert_eq!(config.bitcoin.username, "testuser");
        assert_eq!(config.bitcoin.password, "testpass");
    }
}
