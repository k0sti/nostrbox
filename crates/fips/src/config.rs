//! Generate FIPS YAML config from NostrBox configuration.

use nostrbox_core::FipsConfig;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::FipsError;

/// The generated FIPS daemon config (written to /etc/fips/fips.yaml).
#[derive(Debug, Serialize)]
struct GeneratedFipsConfig {
    node: GeneratedNodeConfig,
    transports: GeneratedTransportsConfig,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    peers: Vec<GeneratedPeerConfig>,
}

#[derive(Debug, Serialize)]
struct GeneratedNodeConfig {
    identity: GeneratedIdentityConfig,
    control: GeneratedControlConfig,
}

#[derive(Debug, Serialize)]
struct GeneratedIdentityConfig {
    persistent: bool,
}

#[derive(Debug, Serialize)]
struct GeneratedControlConfig {
    enabled: bool,
    socket_path: String,
}

#[derive(Debug, Serialize)]
struct GeneratedTransportsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    udp: Option<GeneratedTransportInstance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tcp: Option<GeneratedTransportInstance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ble: Option<GeneratedBleConfig>,
}

#[derive(Debug, Serialize)]
struct GeneratedTransportInstance {
    instances: Vec<GeneratedListenInstance>,
}

#[derive(Debug, Serialize)]
struct GeneratedListenInstance {
    listen: String,
}

#[derive(Debug, Serialize)]
struct GeneratedBleConfig {
    instances: Vec<GeneratedBleInstance>,
}

#[derive(Debug, Serialize)]
struct GeneratedBleInstance {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct GeneratedPeerConfig {
    npub: String,
    addresses: Vec<GeneratedPeerAddress>,
}

#[derive(Debug, Serialize)]
struct GeneratedPeerAddress {
    transport: String,
    addr: String,
}

/// Generate FIPS config YAML and write to disk.
pub fn generate_fips_config(config: &FipsConfig, output_path: &Path) -> Result<(), FipsError> {
    let udp = if config.transports.contains(&"udp".to_string()) {
        Some(GeneratedTransportInstance {
            instances: vec![GeneratedListenInstance {
                listen: config.listen.clone(),
            }],
        })
    } else {
        None
    };

    let tcp = if config.transports.contains(&"tcp".to_string()) {
        Some(GeneratedTransportInstance {
            instances: vec![GeneratedListenInstance {
                listen: config.listen.clone(),
            }],
        })
    } else {
        None
    };

    let ble = if config.transports.contains(&"ble".to_string()) {
        Some(GeneratedBleConfig {
            instances: vec![GeneratedBleInstance { enabled: true }],
        })
    } else {
        None
    };

    // Parse peers: "npub1...@192.168.1.1:2121/udp"
    let peers = config
        .peers
        .iter()
        .filter_map(|p| parse_peer_string(p))
        .collect();

    let generated = GeneratedFipsConfig {
        node: GeneratedNodeConfig {
            identity: GeneratedIdentityConfig { persistent: true },
            control: GeneratedControlConfig {
                enabled: true,
                socket_path: config.socket_path.clone(),
            },
        },
        transports: GeneratedTransportsConfig { udp, tcp, ble },
        peers,
    };

    let yaml = serde_yaml::to_string(&generated)
        .map_err(|e| FipsError::Config(format!("failed to serialize FIPS config: {e}")))?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            FipsError::Config(format!(
                "failed to create config dir {}: {e}",
                parent.display()
            ))
        })?;
    }

    fs::write(output_path, &yaml).map_err(|e| {
        FipsError::Config(format!(
            "failed to write {}: {e}",
            output_path.display()
        ))
    })?;

    tracing::info!(path = %output_path.display(), "wrote FIPS config");
    Ok(())
}

/// Parse "npub1...@192.168.1.1:2121/udp" into a peer config.
fn parse_peer_string(s: &str) -> Option<GeneratedPeerConfig> {
    let (npub, rest) = s.split_once('@')?;
    let (addr, transport) = rest.rsplit_once('/')?;
    Some(GeneratedPeerConfig {
        npub: npub.to_string(),
        addresses: vec![GeneratedPeerAddress {
            transport: transport.to_string(),
            addr: addr.to_string(),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_peer_string() {
        let peer = parse_peer_string("npub1abc@192.168.1.1:2121/udp").unwrap();
        assert_eq!(peer.npub, "npub1abc");
        assert_eq!(peer.addresses[0].transport, "udp");
        assert_eq!(peer.addresses[0].addr, "192.168.1.1:2121");
    }

    #[test]
    fn test_generate_config() {
        let config = FipsConfig {
            enable: true,
            listen: "0.0.0.0:2121".into(),
            transports: vec!["udp".into(), "tcp".into()],
            peers: vec!["npub1test@10.0.0.1:2121/udp".into()],
            socket_path: "/run/fips/control.sock".into(),
            dns_enable: true,
        };

        let dir = tempfile::TempDir::new().unwrap();
        let out = dir.path().join("fips.yaml");
        generate_fips_config(&config, &out).unwrap();

        let contents = fs::read_to_string(&out).unwrap();
        assert!(contents.contains("persistent: true"));
        assert!(contents.contains("control.sock"));
        assert!(contents.contains("npub1test"));
    }
}
