use clap::{Parser, Subcommand};
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod chains;
mod keystore;
mod relay;
mod metrics;
mod cosmwasm;

use config::RelayerConfig;

#[derive(Parser)]
#[command(name = "relayer")]
#[command(about = "IBC Relayer for NEAR-Cosmos bridge")]
#[command(version)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/relayer.toml")]
    pub config: String,

    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the relayer
    Start {
        /// Chains to relay between (comma-separated)
        #[arg(long)]
        chains: Option<String>,
    },
    /// Start CosmWasm execution relayer
    StartCosmWasm {
        /// Path to CosmWasm relayer config
        #[arg(long, default_value = "config/cosmwasm.toml")]
        config: String,
    },
    /// Query chain information
    Query {
        /// Chain identifier
        #[arg(long)]
        chain: String,
        /// Query type
        #[arg(long)]
        query_type: String,
    },
    /// Create IBC connection
    CreateConnection {
        /// Source chain
        #[arg(long)]
        src_chain: String,
        /// Destination chain  
        #[arg(long)]
        dst_chain: String,
    },
    /// Create IBC channel
    CreateChannel {
        /// Connection identifier
        #[arg(long)]
        connection: String,
        /// Port identifier
        #[arg(long)]
        port: String,
    },
    /// Show relayer status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}={}", env!("CARGO_PKG_NAME"), cli.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting IBC Relayer for NEAR-Cosmos bridge");

    // Load configuration
    let config = RelayerConfig::load(&cli.config)?;
    info!("Loaded configuration from: {}", cli.config);

    match cli.command {
        Commands::Start { chains } => {
            info!("Starting relayer...");
            if let Some(chains) = chains {
                info!("Relaying between chains: {}", chains);
            }
            start_relayer(config).await?;
        }
        Commands::StartCosmWasm { config: config_path } => {
            info!("Starting CosmWasm relayer...");
            start_cosmwasm_relayer(&config_path).await?;
        }
        Commands::Query { chain, query_type } => {
            info!("Querying {} on chain {}", query_type, chain);
            query_chain(&config, &chain, &query_type).await?;
        }
        Commands::CreateConnection { src_chain, dst_chain } => {
            info!("Creating connection between {} and {}", src_chain, dst_chain);
            create_connection(&config, &src_chain, &dst_chain).await?;
        }
        Commands::CreateChannel { connection, port } => {
            info!("Creating channel on connection {} for port {}", connection, port);
            create_channel(&config, &connection, &port).await?;
        }
        Commands::Status => {
            info!("Checking relayer status...");
            show_status(&config).await?;
        }
    }

    Ok(())
}

async fn start_relayer(config: RelayerConfig) -> anyhow::Result<()> {
    use crate::chains::Chain;
    use crate::relay::{RelayEngine, create_client_update_manager};
    use crate::metrics::RelayerMetrics;
    use std::collections::HashMap;
    use std::sync::Arc;
    
    info!("🚀 IBC Relayer starting...");
    
    // Initialize metrics
    let metrics = Arc::new(RelayerMetrics::new()?);
    info!("Metrics initialized");
    
    // Initialize chains
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    
    // Check if metrics are enabled before moving config
    let metrics_enabled = config.metrics.enabled;
    let metrics_host = config.metrics.host.clone();
    let metrics_port = config.metrics.port;
    
    // Create client update manager
    let mut client_manager = create_client_update_manager(&config, chains.clone());
    info!("Client update manager initialized with {} client mappings", 
          client_manager.client_mappings_count());
    
    // Start packet relay engine
    let relay_engine = RelayEngine::new(config, chains, metrics.clone());
    info!("Relay engine initialized with {} chains", relay_engine.chains.len());
    
    // Start metrics server if enabled
    if metrics_enabled {
        let registry = metrics.registry();
        info!("Metrics server would start on {}:{}", metrics_host, metrics_port);
        // In real implementation, would start HTTP server here
        let _ = registry.gather(); // Use registry
    }
    
    // Start client update service in background
    tokio::spawn(async move {
        if let Err(e) = client_manager.start().await {
            error!("Client update manager failed: {}", e);
        }
    });
    
    info!("✅ All services started successfully");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down relayer...");
    
    Ok(())
}

async fn query_chain(_config: &RelayerConfig, chain: &str, query_type: &str) -> anyhow::Result<()> {
    info!("Query {} on {}: Not implemented yet", query_type, chain);
    Ok(())
}

async fn create_connection(_config: &RelayerConfig, src_chain: &str, dst_chain: &str) -> anyhow::Result<()> {
    info!("Create connection {}->{}: Not implemented yet", src_chain, dst_chain);
    Ok(())
}

async fn create_channel(_config: &RelayerConfig, connection: &str, port: &str) -> anyhow::Result<()> {
    info!("Create channel on {} for {}: Not implemented yet", connection, port);
    Ok(())
}

async fn start_cosmwasm_relayer(config_path: &str) -> anyhow::Result<()> {
    use crate::cosmwasm::{CosmWasmRelayerService, CosmWasmRelayerConfig};
    use std::fs;
    
    info!("🚀 Starting CosmWasm execution relayer");
    info!("Loading configuration from: {}", config_path);
    
    // Load configuration
    let config_content = fs::read_to_string(config_path)?;
    let config: CosmWasmRelayerConfig = toml::from_str(&config_content)?;
    
    info!("Configuration loaded:");
    info!("  NEAR RPC: {}", config.near_rpc_url);
    info!("  Relayer account: {}", config.relayer_account_id);
    info!("  WASM module contract: {}", config.wasm_module_contract);
    info!("  Polling interval: {}ms", config.polling_interval_ms);
    
    // Create and start the service
    let service = CosmWasmRelayerService::new(config);
    
    info!("✅ CosmWasm relayer service initialized");
    info!("Starting monitoring and execution workers...");
    
    // Run the service
    service.start().await?;
    
    Ok(())
}

async fn show_status(config: &RelayerConfig) -> anyhow::Result<()> {
    use crate::relay::create_client_update_manager;
    use std::collections::HashMap;
    use std::sync::Arc;
    use crate::chains::Chain;
    
    info!("📊 Relayer Status Report");
    
    // Initialize minimal setup for status checking
    let chains: HashMap<String, Arc<dyn Chain>> = HashMap::new();
    let client_manager = create_client_update_manager(config, chains);
    
    // Show configuration
    info!("🔧 Configuration:");
    info!("  - Chains configured: {}", config.chains.len());
    info!("  - Connections configured: {}", config.connections.len());
    info!("  - Metrics enabled: {}", config.metrics.enabled);
    
    // Show client mappings
    info!("🔗 Client Mappings:");
    for (chain_id, client_id) in client_manager.client_mappings() {
        info!("  - Chain '{}' -> Client '{}'", chain_id, client_id);
    }
    
    // Show connection status
    info!("🌐 Connection Status:");
    for connection in &config.connections {
        info!("  - Connection '{}': {} ↔ {} (auto_relay: {})", 
              connection.id, connection.src_chain, connection.dst_chain, connection.auto_relay);
        info!("    Source client: {}, Dest client: {}", 
              connection.src_client_id, connection.dst_client_id);
    }
    
    // If we had active chains, we could show client status here
    // let statuses = client_manager.get_status().await;
    // info!("📊 Client Status:");
    // for status in statuses {
    //     info!("  - Client '{}' ({}): height {} -> {} (lag: {}, needs_update: {})",
    //           status.client_id, status.chain_id, status.last_updated_height,
    //           status.source_height, status.block_lag, status.needs_update);
    // }
    
    info!("✅ Status check complete");
    Ok(())
}