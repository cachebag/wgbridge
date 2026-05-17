use log::info;
use std::process::Command;

/// Enable IPv4 forwarding so the Pi can route packets.
pub fn enable_ip_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    info!("Enabling IPv4 forwarding");

    std::fs::write("/proc/sys/net/ipv4/ip_forward", "1")?;

    // Make it persistent across reboots
    let sysctl_conf = "/etc/sysctl.d/99-wgbridge.conf";
    std::fs::write(sysctl_conf, "net.ipv4.ip_forward = 1\n")?;

    info!("IPv4 forwarding enabled");
    Ok(())
}

/// Set up NAT masquerade rules so traffic from the LAN interface
/// gets routed through the WireGuard tunnel.
pub fn setup_nat(lan_interface: &str, wg_interface: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Setting up NAT: {} -> {}",
        lan_interface, wg_interface
    );

    // Flush any existing wgbridge rules
    let _ = run_iptables(&[
        "-t", "nat", "-D", "POSTROUTING",
        "-o", wg_interface,
        "-j", "MASQUERADE",
    ]);
    let _ = run_iptables(&[
        "-D", "FORWARD",
        "-i", lan_interface,
        "-o", wg_interface,
        "-j", "ACCEPT",
    ]);
    let _ = run_iptables(&[
        "-D", "FORWARD",
        "-i", wg_interface,
        "-o", lan_interface,
        "-m", "state", "--state", "RELATED,ESTABLISHED",
        "-j", "ACCEPT",
    ]);

    // Masquerade outgoing traffic on the WireGuard interface
    run_iptables(&[
        "-t", "nat", "-A", "POSTROUTING",
        "-o", wg_interface,
        "-j", "MASQUERADE",
    ])?;

    // Allow forwarding from LAN to WireGuard
    run_iptables(&[
        "-A", "FORWARD",
        "-i", lan_interface,
        "-o", wg_interface,
        "-j", "ACCEPT",
    ])?;

    // Allow established/related connections back
    run_iptables(&[
        "-A", "FORWARD",
        "-i", wg_interface,
        "-o", lan_interface,
        "-m", "state", "--state", "RELATED,ESTABLISHED",
        "-j", "ACCEPT",
    ])?;

    info!("NAT rules configured");
    Ok(())
}

/// Persist iptables rules so they survive reboot.
pub fn persist_iptables() -> Result<(), Box<dyn std::error::Error>> {
    info!("Persisting iptables rules");

    let output = Command::new("iptables-save").output()?;
    if !output.status.success() {
        return Err("Failed to save iptables rules".into());
    }

    std::fs::write("/etc/iptables/iptables.rules", &output.stdout)?;

    // Enable iptables service so rules load on boot
    let _ = Command::new("systemctl")
        .args(["enable", "iptables"])
        .output();

    info!("iptables rules persisted");
    Ok(())
}

fn run_iptables(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("iptables").args(args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("iptables failed: {}", stderr).into());
    }

    Ok(())
}
