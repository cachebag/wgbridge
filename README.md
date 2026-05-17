# wgbridge

Transparent WireGuard gateway for Raspberry Pi.

My father-in-law bought this TV box when he was back home in Mexico, but of course in the US, none of the content is accessible.

<img width="5712" height="4284" alt="XUPER" src="https://github.com/user-attachments/assets/db71fdf0-80e4-48c1-9750-91e2ae772eee" />


`wgbridge` is a transparent WireGuard gateway for Raspberry Pi. Helps our case out pretty well.

It turns out that this can work pretty well for other IPTV boxes, streaming appliances, smart TVs, game consoles, and similar hardware.

All NM stuff is handled using [nmrs](https://github.com/networkmanager-rs/nmrs).

## Architecture

It's quite simple:

```text
Device -> Raspberry Pi -> WireGuard tunnel -> VPS -> Internet
```

Example:

```text
Google TV Box -> Pi 5 -> VPS in Mexico City -> Mexican IP
```

## Requirements

* Raspberry Pi running Linux
* NetworkManager
* WireGuard
* Rust toolchain
* A VPS or WireGuard endpoint

This is currently tested on _my_ specific use case, so YMMV:

* Raspberry Pi 5
* Arch Linux ARM
* Ubuntu VPS

## Installation

Clone the repository:

```bash
git clone git@github.com:cachebag/wgbridge.git
cd wgbridge
```

Build:

```bash
cargo build --release
```

Binary:

```bash
./target/release/wgbridge
```

or you can probably do:

```bash
cargo install --path .
```

and you can get `wgbridge` globally.

## Quick Start

Initial setup:

```bash
wgbridge setup \
  --wifi-ssid "MyWiFi" \
  --wifi-password "password" \
  --private-key "<client-private-key>" \
  --address "10.0.0.2/24" \
  --peer-public-key "<server-public-key>" \
  --peer-endpoint "1.2.3.4:51820"
```

Bring gateway online:

```bash
wgbridge up
```

Check status:

```bash
wgbridge status
```

Update Wi-Fi only:

```bash
wgbridge wifi \
  --ssid "NewWiFi" \
  --password "NewPassword"
```

## Policy Routing

Typical deployment:

```bash
ip rule add from 192.168.1.118/32 lookup 52299
```

This routes only the selected device through the WireGuard tunnel while keeping normal Pi traffic local.

## Systemd

Example service:

```ini
[Unit]
Description=wgbridge startup
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/wgbridge up
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

## Roadmap 

I am most definitely going to abandon this project. But if I don't, I think it would be cool to add this stuff:

* Automatic LAN interface detection
* Automatic policy-route management
* DNS leak prevention
* DHCP mode
* IPv6 support
* Web UI
* Multiple device profiles
* Containerized deployment
* Travel-router mode


## License

MIT
