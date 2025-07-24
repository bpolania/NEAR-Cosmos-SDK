use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub global: GlobalConfig,
    pub chains: HashMap<String, ChainConfig>,
    pub connections: Vec<ConnectionConfig>,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Log level for the relayer
    pub log_level: String,
    /// Maximum number of retries for failed operations
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain type: "near" or "cosmos"
    pub chain_type: String,
    /// Chain identifier
    pub chain_id: String,
    /// RPC endpoint
    pub rpc_endpoint: String,
    /// WebSocket endpoint for events (optional)
    pub ws_endpoint: Option<String>,
    /// Chain-specific configuration
    pub config: ChainSpecificConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChainSpecificConfig {
    #[serde(rename = "near")]
    Near {
        /// NEAR contract account ID (our Cosmos SDK contract)
        contract_id: String,
        /// Account ID for signing transactions
        signer_account_id: String,
        /// Private key for signing (in development)
        private_key: Option<String>,
        /// Network ID (mainnet, testnet, localnet)
        network_id: String,
    },
    #[serde(rename = "cosmos")]
    Cosmos {
        /// Bech32 address prefix
        address_prefix: String,
        /// Gas price and denom
        gas_price: String,
        /// Trust threshold for light client
        trust_threshold: String,
        /// Trusting period in hours
        trusting_period_hours: u64,
        /// Account for signing transactions
        signer_key: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection identifier
    pub id: String,
    /// Source chain
    pub src_chain: String,
    /// Destination chain
    pub dst_chain: String,
    /// Source client ID
    pub src_client_id: String,
    /// Destination client ID
    pub dst_client_id: String,
    /// Auto-relay packets on this connection
    pub auto_relay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics server
    pub enabled: bool,
    /// Metrics server host
    pub host: String,
    /// Metrics server port
    pub port: u16,
}

impl RelayerConfig {
    /// Load configuration from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: RelayerConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get chain configuration by ID
    pub fn get_chain(&self, chain_id: &str) -> Option<&ChainConfig> {
        self.chains.get(chain_id)
    }

    /// Get connection configuration by ID
    pub fn get_connection(&self, connection_id: &str) -> Option<&ConnectionConfig> {
        self.connections.iter().find(|c| c.id == connection_id)
    }
}

impl Default for RelayerConfig {
    fn default() -> Self {
        let mut chains = HashMap::new();
        
        // Default NEAR chain configuration (our contract)
        chains.insert("near-testnet".to_string(), ChainConfig {
            chain_type: "near".to_string(),
            chain_id: "near-testnet".to_string(),
            rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
            ws_endpoint: None,
            config: ChainSpecificConfig::Near {
                contract_id: "cosmos-sdk-demo.testnet".to_string(),
                signer_account_id: "relayer.testnet".to_string(),
                private_key: None,
                network_id: "testnet".to_string(),
            },
        });

        // Default Cosmos chain configuration
        chains.insert("cosmoshub-testnet".to_string(), ChainConfig {
            chain_type: "cosmos".to_string(),
            chain_id: "theta-testnet-001".to_string(),
            rpc_endpoint: "https://rpc.sentry-01.theta-testnet.polypore.xyz".to_string(),
            ws_endpoint: Some("wss://rpc.sentry-01.theta-testnet.polypore.xyz/websocket".to_string()),
            config: ChainSpecificConfig::Cosmos {
                address_prefix: "cosmos".to_string(),
                gas_price: "0.025uatom".to_string(),
                trust_threshold: "1/3".to_string(),
                trusting_period_hours: 336, // 14 days
                signer_key: None,
            },
        });

        Self {
            global: GlobalConfig {
                log_level: "info".to_string(),
                max_retries: 3,
                retry_delay_ms: 1000,
                health_check_interval: 30,
            },
            chains,
            connections: vec![],
            metrics: MetricsConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 3001,
            },
        }
    }
}