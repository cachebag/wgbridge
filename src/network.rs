use crate::config::{Config, WireGuardSettings};
use log::{error, info};
use nmrs::{NetworkManager, WifiSecurity, WireGuardConfig, WireGuardPeer};

/// Connect to a Wi-Fi network using nmrs.
pub async fn connect_wifi(
    nm: &NetworkManager,
    ssid: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to Wi-Fi network: {}", ssid);

    nm.connect(
        ssid,
        None,
        WifiSecurity::WpaPsk {
            psk: password.into(),
        },
    )
    .await?;

    if let Some(current) = nm.current_ssid().await {
        info!("Connected to Wi-Fi: {}", current);
    }

    Ok(())
}

/// Bring up the WireGuard tunnel using nmrs.
pub async fn connect_wireguard(
    nm: &NetworkManager,
    wg: &WireGuardSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Establishing WireGuard tunnel: {}", wg.name);

    let peer = WireGuardPeer::new(
        &wg.peer_public_key,
        &wg.peer_endpoint,
        wg.allowed_ips.clone(),
    )
    .with_persistent_keepalive(wg.persistent_keepalive.unwrap_or(25));

    let mut config = WireGuardConfig::new(
        &wg.name,
        &wg.peer_endpoint,
        &wg.private_key,
        &wg.address,
        vec![peer],
    );

    if !wg.dns.is_empty() {
        config = config.with_dns(wg.dns.clone());
    }

    nm.connect_vpn(config).await?;

    info!("WireGuard tunnel '{}' is up", wg.name);
    Ok(())
}

/// Bring up the full networking stack: Wi-Fi (if configured) + WireGuard.
pub async fn bring_up(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let nm = NetworkManager::new().await?;

    // Step 1: Connect to Wi-Fi if configured
    if let Some(ref wifi) = config.wifi {
        connect_wifi(&nm, &wifi.ssid, &wifi.password).await?;
    } else {
        info!("No Wi-Fi configured, assuming wired connection");
    }

    // Step 2: Bring up WireGuard tunnel
    connect_wireguard(&nm, &config.wireguard).await?;

    Ok(())
}

/// Tear down the WireGuard tunnel.
pub async fn tear_down(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let nm = NetworkManager::new().await?;

    info!("Disconnecting WireGuard tunnel: {}", config.wireguard.name);
    nm.disconnect_vpn(&config.wireguard.name).await?;

    info!("Tunnel disconnected");
    Ok(())
}
