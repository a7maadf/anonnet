/// AnonNet Daemon - Anonymous Network Node
///
/// This daemon runs an AnonNet node that:
/// - Connects to the P2P network via DHT
/// - Creates and relays anonymous circuits
/// - Manages credits and transactions
/// - Provides SOCKS5 and HTTP proxy services

use anyhow::Result;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber;

use anonnet_daemon::ProxyManager;

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
                info!("Full node mode not yet implemented");
                info!("For now, use 'proxy' mode to run proxy services");
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

    // Default proxy addresses
    let socks5_addr: SocketAddr = "127.0.0.1:9050".parse()?;
    let http_addr: SocketAddr = "127.0.0.1:8118".parse()?;

    info!("SOCKS5 proxy will listen on: {}", socks5_addr);
    info!("HTTP proxy will listen on: {}", http_addr);

    // Create and start proxy manager
    let proxy_manager = ProxyManager::new(socks5_addr, http_addr);
    proxy_manager.start().await?;

    Ok(())
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
    println!("    node        Run full AnonNet node (coming soon)");
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
