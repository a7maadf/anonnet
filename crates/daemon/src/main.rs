/// AnonNet Daemon - Anonymous Network Node
///
/// This daemon runs an AnonNet node that:
/// - Connects to the P2P network via DHT
/// - Creates and relays anonymous circuits
/// - Manages credits and transactions
/// - Provides SOCKS5 and HTTP proxy services

use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber;

use anonnet_core::{Node, NodeStats};
use anonnet_common::NodeConfig;
use anonnet_daemon::{ApiServer, ProxyManager};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting AnonNet Daemon v{}", env!("CARGO_PKG_VERSION"));

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "help" | "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "version" | "--version" | "-v" => {
                println!("AnonNet Daemon v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "proxy" => {
                run_proxy_mode().await?;
            }
            "node" => {
                run_node_mode().await?;
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                eprintln!("Run with 'help' to see available commands");
                std::process::exit(1);
            }
        }
    } else {
        // Default: run proxy mode
        run_proxy_mode().await?;
    }

    Ok(())
}

/// Run in proxy mode (SOCKS5 + HTTP proxy)
async fn run_proxy_mode() -> Result<()> {
    info!("Running in proxy mode");

    // Create a lightweight node for proxy services
    info!("Creating AnonNet node for proxy services...");
    let config = NodeConfig::default();
    let mut node = Node::new(config).await?;

    // Start the node
    node.start().await?;
    info!("Node started");

    // Wrap in Arc for sharing with proxies
    let node = Arc::new(node);

    // Default proxy addresses
    let socks5_addr: SocketAddr = "127.0.0.1:9050".parse()?;
    let http_addr: SocketAddr = "127.0.0.1:8118".parse()?;
    let api_addr: SocketAddr = "127.0.0.1:9150".parse()?; // 9051 is Tor's control port

    info!("SOCKS5 proxy will listen on: {}", socks5_addr);
    info!("HTTP proxy will listen on: {}", http_addr);
    info!("API server will listen on: {}", api_addr);

    // Start API server in background
    let api_server = ApiServer::new(api_addr, node.clone());
    tokio::spawn(async move {
        if let Err(e) = api_server.start().await {
            warn!("API server error: {}", e);
        }
    });

    // Create and start proxy manager with node
    let proxy_manager = ProxyManager::new(socks5_addr, http_addr, node.clone());
    proxy_manager.start().await?;

    Ok(())
}

/// Run in full node mode
async fn run_node_mode() -> Result<()> {
    info!("Running in full node mode");

    // Load or create default configuration
    let config_path = PathBuf::from("anonnet.toml");
    let config = if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        NodeConfig::from_file(&config_path)?
    } else {
        info!("No configuration file found, using defaults");
        let config = NodeConfig::default();

        // Save default config for next time
        if let Err(e) = config.to_file(&config_path) {
            warn!("Failed to save default config: {}", e);
        } else {
            info!("Saved default configuration to {:?}", config_path);
        }

        config
    };

    // Create and start node
    info!("Creating AnonNet node...");
    let mut node = Node::new(config).await?;

    info!("Starting node...");
    node.start().await?;

    // Print node stats
    let stats = node.get_stats().await;
    print_node_stats(&stats);

    // Keep node running
    info!("Node is running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    info!("Shutdown signal received");
    node.stop().await?;

    info!("Node stopped");
    Ok(())
}

/// Print node statistics
fn print_node_stats(stats: &NodeStats) {
    println!("\n========================================");
    println!("         AnonNet Node Status");
    println!("========================================");
    println!("Node ID:          {}", stats.node_id);
    println!("Status:           {}", if stats.is_running { "Running" } else { "Stopped" });
    println!("Peers:            {}", stats.peer_count);
    println!("Active Peers:     {}", stats.active_peers);
    println!("Circuits:         {}", stats.circuits);
    println!("Active Circuits:  {}", stats.active_circuits);
    println!("Bandwidth:        {} bytes/sec", stats.bandwidth);
    println!("========================================\n");
}

/// Print help message
fn print_help() {
    println!("AnonNet Daemon - Anonymous Network Node");
    println!();
    println!("USAGE:");
    println!("    anonnet-daemon [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    proxy       Run SOCKS5 and HTTP proxy services (default)");
    println!("    node        Run full AnonNet node with DHT, circuits, and consensus");
    println!("    help        Show this help message");
    println!("    version     Show version information");
    println!();
    println!("PROXY MODE:");
    println!("    SOCKS5:     127.0.0.1:9050  (Tor-compatible)");
    println!("    HTTP:       127.0.0.1:8118  (Privoxy-compatible)");
    println!();
    println!("EXAMPLES:");
    println!("    # Start proxy services");
    println!("    anonnet-daemon");
    println!("    anonnet-daemon proxy");
    println!();
    println!("    # Configure browser:");
    println!("    SOCKS5: localhost:9050");
    println!("    HTTP:   localhost:8118");
    println!();
    println!("    # Use with curl:");
    println!("    curl --proxy socks5h://localhost:9050 https://example.com");
    println!("    curl --proxy http://localhost:8118 http://example.com");
}
