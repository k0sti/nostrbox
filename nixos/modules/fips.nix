{ config, lib, pkgs, inputs, ... }:

let
  cfg = config.services.fips;

  # Generate FIPS YAML config
  fipsConfigFile = pkgs.writeText "fips.yaml" (builtins.toJSON {
    node = {
      identity = {
        persistent = true;
      };
    };
    tun = {
      enabled = cfg.tun.enable;
      name = cfg.tun.name;
      mtu = cfg.tun.mtu;
    };
    dns = {
      enabled = cfg.dns.enable;
      bind_addr = cfg.dns.bindAddr;
    };
    transports = lib.filterAttrs (_: v: v != null) {
      udp = if builtins.elem "udp" cfg.transports then {
        bind_addr = cfg.listenAddress;
      } else null;
      tcp = if builtins.elem "tcp" cfg.transports then {
        bind_addr = cfg.listenAddress;
      } else null;
      ethernet = if cfg.ethernet.enable then {
        interface = cfg.ethernet.interface;
        discovery = cfg.ethernet.discovery;
        announce = cfg.ethernet.announce;
        auto_connect = cfg.ethernet.autoConnect;
        accept_connections = cfg.ethernet.acceptConnections;
      } else null;
      ble = if builtins.elem "ble" cfg.transports then {
        instances = [{ enabled = true; }];
      } else null;
    };
    peers = cfg.peers;
    control = {
      enabled = true;
      socket_path = cfg.socketPath;
    };
  });
in {
  options.services.fips = {
    enable = lib.mkEnableOption "FIPS mesh networking daemon";

    package = lib.mkOption {
      type = lib.types.package;
      default = inputs.fips.packages.${pkgs.system}.fips-ble;
      defaultText = "inputs.fips.packages.\${system}.fips-ble";
      description = "FIPS package (use fips-ble for BLE support)";
    };

    keyDir = lib.mkOption {
      type = lib.types.path;
      default = "/etc/fips";
      description = "Directory containing fips.key and fips.pub";
    };

    listenAddress = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0:2121";
      description = "FIPS transport listen address (for udp/tcp)";
    };

    transports = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ "udp" ];
      description = "Enabled FIPS transports (udp, tcp, ble). Ethernet is configured separately.";
    };

    ethernet = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable raw Ethernet transport";
      };
      interface = lib.mkOption {
        type = lib.types.str;
        default = "eth0";
        description = "Ethernet interface for raw frame transport";
      };
      discovery = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Listen for discovery beacons";
      };
      announce = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Broadcast announcement beacons on LAN";
      };
      autoConnect = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Auto-connect to discovered peers";
      };
      acceptConnections = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Accept incoming connection attempts";
      };
    };

    peers = lib.mkOption {
      type = lib.types.listOf (lib.types.submodule {
        options = {
          npub = lib.mkOption { type = lib.types.str; description = "Peer npub"; };
          alias = lib.mkOption { type = lib.types.str; default = ""; description = "Human-readable name"; };
          addresses = lib.mkOption {
            type = lib.types.listOf (lib.types.submodule {
              options = {
                transport = lib.mkOption { type = lib.types.str; default = "udp"; };
                addr = lib.mkOption { type = lib.types.str; description = "host:port"; };
              };
            });
            description = "Transport addresses for this peer";
          };
        };
      });
      default = [];
      description = "Static FIPS peers";
    };

    socketPath = lib.mkOption {
      type = lib.types.str;
      default = "/run/fips/control.sock";
      description = "FIPS control socket path";
    };

    tun = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Enable TUN interface";
      };
      name = lib.mkOption {
        type = lib.types.str;
        default = "fips0";
        description = "TUN interface name";
      };
      mtu = lib.mkOption {
        type = lib.types.int;
        default = 1280;
        description = "TUN interface MTU";
      };
    };

    dns = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Enable FIPS DNS responder";
      };
      bindAddr = lib.mkOption {
        type = lib.types.str;
        default = "127.0.0.1";
        description = "DNS responder bind address";
      };
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Open firewall for FIPS transport port";
    };
  };

  config = lib.mkIf cfg.enable {
    # FIPS systemd service
    systemd.services.fips = {
      description = "FIPS Mesh Network Daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/fips -c ${fipsConfigFile}";

        # FIPS needs CAP_NET_ADMIN for TUN interface creation
        AmbientCapabilities = [ "CAP_NET_ADMIN" ];
        CapabilityBoundingSet = [ "CAP_NET_ADMIN" ];

        DynamicUser = false;

        # Control socket directory
        RuntimeDirectory = "fips";
        RuntimeDirectoryMode = "0755";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        ReadWritePaths = [
          cfg.keyDir
          "/run/fips"
        ];

        Restart = "on-failure";
        RestartSec = 5;
      };

      environment = {
        RUST_LOG = "info,fips=debug";
      };
    };

    # Firewall
    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedUDPPorts = lib.optionals (builtins.elem "udp" cfg.transports) [
        (lib.toInt (lib.last (lib.splitString ":" cfg.listenAddress)))
      ];
      allowedTCPPorts = lib.optionals (builtins.elem "tcp" cfg.transports) [
        (lib.toInt (lib.last (lib.splitString ":" cfg.listenAddress)))
      ];
    };
  };
}
