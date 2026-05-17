use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for the WireGuard gateway bridge.
///
/// This gets persisted to disk so the Pi can reconstruct
/// its full networking state on boot without any manual input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub wifi: Option<WifiConfig>,
    pub wireguard: WireGuardSettings,
    pub gateway: GatewayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardSettings {
    pub name: String,
    pub private_key: String,
    pub address: String,
    pub dns: Vec<String>,
    pub peer_public_key: String,
    pub peer_endpoint: String,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// The network interface to forward traffic from (e.g., "eth0", "wlan0")
    pub lan_interface: String,
    /// The WireGuard interface name (usually "wg0" or the NM connection name)
    pub wg_interface: String,
}

impl GatewayConfig {
    pub fn resolved_wg_interface(&self) -> String {
        if self.wg_interface.starts_with("wg-") {
            self.wg_interface.clone()
        } else {
            format!("wg-{}", self.wg_interface)
        }
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        PathBuf::from("/etc/wgbridge/config.toml")
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
