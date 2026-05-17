mod config;
mod network;
mod routing;

use clap::{Parser, Subcommand};
use config::Config;
use log::{error, info};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "wgbridge",
    about = "Turn a Raspberry Pi into a WireGuard gateway for geo-restricted devices",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file
    #[arg(short, long, default_value = "/etc/wgbridge/config.toml")]
    config: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Initial setup: configure Wi-Fi, WireGuard, and routing
    Setup {
        /// Wi-Fi SSID to connect to
        #[arg(long)]
        wifi_ssid: Option<String>,

        /// Wi-Fi password
        #[arg(long)]
        wifi_password: Option<String>,

        /// WireGuard private key
        #[arg(long)]
        private_key: String,

        /// WireGuard client address (e.g., "10.0.0.2/24")
        #[arg(long)]
        address: String,

        /// WireGuard peer public key (server)
        #[arg(long)]
        peer_public_key: String,

        /// WireGuard peer endpoint (e.g., "1.2.3.4:51820")
        #[arg(long)]
        peer_endpoint: String,

        /// DNS servers (comma-separated)
        #[arg(long, default_value = "1.1.1.1")]
        dns: String,

        /// LAN interface to forward traffic from
        #[arg(long, default_value = "end0")]
        lan_interface: String,

        /// Persistent keepalive interval in seconds
        #[arg(long, default_value = "25")]
        keepalive: u32,
    },

    /// Bring up the gateway (Wi-Fi + WireGuard + routing)
    Up,

    /// Tear down the gateway
    Down,

    /// Show current status
    Status,

    /// Update Wi-Fi credentials only
    Wifi {
        /// Wi-Fi SSID
        #[arg(long)]
        ssid: String,

        /// Wi-Fi password
        #[arg(long)]
        password: String,
    },
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Setup {
            wifi_ssid,
            wifi_password,
            private_key,
            address,
            peer_public_key,
            peer_endpoint,
            dns,
            lan_interface,
            keepalive,
        } => {
            run_setup(
                &cli.config,
                wifi_ssid,
                wifi_password,
                private_key,
                address,
                peer_public_key,
                peer_endpoint,
                dns,
                lan_interface,
                keepalive,
            )
            .await
        }
        Commands::Up => run_up(&cli.config).await,
        Commands::Down => run_down(&cli.config).await,
        Commands::Status => run_status(&cli.config).await,
        Commands::Wifi { ssid, password } => {
            run_wifi(&cli.config, ssid, password).await
        }
    };

    if let Err(e) = result {
        error!("Fatal: {}", e);
        std::process::exit(1);
    }
}

async fn run_setup(
    config_path: &PathBuf,
    wifi_ssid: Option<String>,
    wifi_password: Option<String>,
    private_key: String,
    address: String,
    peer_public_key: String,
    peer_endpoint: String,
    dns: String,
    lan_interface: String,
    keepalive: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting wgbridge setup...");

    let wifi = match (wifi_ssid, wifi_password) {
        (Some(ssid), Some(password)) => Some(config::WifiConfig { ssid, password }),
        (None, None) => None,
        _ => return Err("Both --wifi-ssid and --wifi-password must be provided together".into()),
    };

    // Determine the WireGuard interface name NM will create
    let wg_name = "wgbridge0";

    let config = Config {
        wifi,
        wireguard: config::WireGuardSettings {
            name: wg_name.into(),
            private_key,
            address,
            dns: dns.split(',').map(|s| s.trim().to_string()).collect(),
            peer_public_key,
            peer_endpoint,
            allowed_ips: vec!["0.0.0.0/0".into()],
            persistent_keepalive: Some(keepalive),
        },
        gateway: config::GatewayConfig {
            lan_interface: lan_interface.clone(),
            wg_interface: wg_name.into(),
        },
    };

    // Save config
    config.save(config_path)?;
    info!("Config saved to {}", config_path.display());

    // Bring up networking
    network::bring_up(&config).await?;

    // Configure routing
    routing::enable_ip_forwarding()?;
    routing::setup_nat(
        &config.gateway.lan_interface,
        &config.gateway.resolved_wg_interface(),
    )?;
    routing::persist_iptables()?;

    info!("wgbridge setup complete!");
    info!(
        "The TV box should use this Pi's IP as its gateway. \
         All traffic will be routed through the WireGuard tunnel to Mexico."
    );

    Ok(())
}

async fn run_up(config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(config_path)?;

    info!("Bringing up wgbridge...");
    network::bring_up(&config).await?;
    routing::enable_ip_forwarding()?;
    routing::setup_nat(
        &config.gateway.lan_interface,
        &config.gateway.resolved_wg_interface(),
    )?;

    info!("wgbridge is up and routing traffic");
    Ok(())
}

async fn run_down(config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(config_path)?;

    info!("Tearing down wgbridge...");
    network::tear_down(&config).await?;

    info!("wgbridge is down");
    Ok(())
}

async fn run_status(config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(config_path)?;

    let nm = nmrs::NetworkManager::new().await?;

    // Check Wi-Fi
    if let Some(ssid) = nm.current_ssid().await {
        info!("Wi-Fi: connected to '{}'", ssid);
    } else {
        info!("Wi-Fi: not connected");
    }

    // Check VPN
    let vpns = nm.list_vpn_connections().await?;
    let tunnel = vpns.iter().find(|v| v.name == config.wireguard.name);

    match tunnel {
        Some(vpn) => info!("Tunnel '{}': {:?}", vpn.name, vpn.state),
        None => info!("Tunnel '{}': not found", config.wireguard.name),
    }

    // Check IP forwarding
    let fwd = std::fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
        .unwrap_or_default()
        .trim()
        .to_string();
    info!("IP forwarding: {}", if fwd == "1" { "enabled" } else { "disabled" });

    Ok(())
}

async fn run_wifi(
    config_path: &PathBuf,
    ssid: String,
    password: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Updating Wi-Fi configuration...");

    let mut config = Config::load(config_path)?;

    config.wifi = Some(config::WifiConfig {
        ssid: ssid.clone(),
        password: password.clone(),
    });

    config.save(config_path)?;

    let nm = nmrs::NetworkManager::new().await?;

    network::connect_wifi(&nm, &ssid, &password).await?;

    info!("Wi-Fi updated successfully");

    Ok(())
}
