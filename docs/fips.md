# FIPS Configuration

FIPS is a peer-to-peer mesh networking daemon that creates encrypted overlay networks using Nostr identities. It supports multiple transports (UDP, TCP, BLE, raw Ethernet) and provides a TUN interface for IP-level connectivity between peers.

Source: [github.com/k0sti/fips](https://github.com/k0sti/fips)

## NixOS Module

The FIPS NixOS module is defined in `nixos/modules/fips.nix`. It generates a YAML config and runs FIPS as a systemd service.

### Basic usage

```nix
services.fips = {
  enable = true;
  transports = [ "udp" ];
  peers = [
    {
      npub = "npub1...";
      alias = "my-peer";
      addresses = [
        { transport = "udp"; addr = "10.0.0.2:2121"; }
      ];
    }
  ];
};
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `enable` | `false` | Enable FIPS daemon |
| `package` | `fips-ble` | FIPS package to use |
| `configFile` | `null` | Path to a mutable config file (bypasses generated config) |
| `keyDir` | `/etc/fips` | Directory containing `fips.key` and `fips.pub` |
| `listenAddress` | `0.0.0.0:2121` | Transport listen address |
| `transports` | `[ "udp" ]` | Enabled transports: `udp`, `tcp`, `ble` |
| `ethernet.enable` | `false` | Enable raw Ethernet transport |
| `ethernet.interface` | `eth0` | Ethernet interface for raw frames |
| `peers` | `[]` | Static peer list (npub + addresses) |
| `tun.enable` | `true` | Enable TUN interface |
| `tun.name` | `fips0` | TUN interface name |
| `tun.mtu` | `1280` | TUN MTU |
| `dns.enable` | `true` | Enable DNS responder |
| `dns.bindAddr` | `127.0.0.1` | DNS bind address |
| `socketPath` | `/run/fips/control.sock` | Control socket path |
| `openFirewall` | `true` | Auto-open firewall ports |

## Mutable config for development

By default the config is generated into the read-only Nix store. For development you can use a mutable config file instead.

### Setup

1. Seed the config from the Nix-generated version:

```bash
fips-seed-config
# writes ~/.config/fips/fips.yaml (pretty-printed YAML)
```

2. Point the service at it in your machine config:

```nix
services.fips = {
  enable = true;
  configFile = "/home/k0/.config/fips/fips.yaml";
};
```

3. Rebuild once, then iterate by editing the file and restarting:

```bash
vim ~/.config/fips/fips.yaml
sudo systemctl restart fips
```

The `fips-seed-config` script accepts an optional path as first argument and `-f` as second to overwrite an existing file.

## Key management

Keys are stored in `/etc/fips/` by default (`fips.key`, `fips.pub`). The deploy script (`nixos/deploy-nbox.sh`) handles initial key provisioning. Key files should be `0600` and owned by root.
