{ config, lib, pkgs, ... }:

let
  cfg = config.services.nostrbox;
in {
  options.services.nostrbox = {
    enable = lib.mkEnableOption "NostrBox appliance";

    package = lib.mkOption {
      type = lib.types.package;
      description = "NostrBox server package";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/nostrbox";
      description = "Data directory for NostrBox state";
    };

    configFile = lib.mkOption {
      type = lib.types.path;
      default = "/etc/nostrbox/nostrbox.toml";
      description = "Path to NostrBox config file";
    };

    webDistPath = lib.mkOption {
      type = lib.types.path;
      description = "Path to built web UI dist";
    };

    bindAddress = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0:3400";
      description = "Address:port for the management UI/API";
    };

    relayPort = lib.mkOption {
      type = lib.types.port;
      default = 7777;
      description = "Nostr relay WebSocket port";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall for relay and management ports";
    };

    fips = {
      enable = lib.mkEnableOption "FIPS mesh networking";

      package = lib.mkOption {
        type = lib.types.package;
        description = "FIPS package (use fips-ble for BLE support)";
      };

      listenAddress = lib.mkOption {
        type = lib.types.str;
        default = "0.0.0.0:9735";
        description = "FIPS listen address";
      };

      transports = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ "udp" "tcp" ];
        description = "Enabled FIPS transports";
      };

      peers = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [];
        description = "Static FIPS peers (npub@host:port)";
      };

      socketPath = lib.mkOption {
        type = lib.types.str;
        default = "/run/fips/fips.sock";
        description = "FIPS control socket path";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.nostrbox = {
      isSystemUser = true;
      group = "nostrbox";
      home = cfg.dataDir;
      createHome = true;
    };
    users.groups.nostrbox = {};

    systemd.services.nostrbox = {
      description = "NostrBox Server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      serviceConfig = {
        Type = "simple";
        User = "nostrbox";
        Group = "nostrbox";
        ExecStart = "${cfg.package}/bin/nostrbox-server";
        WorkingDirectory = cfg.dataDir;
        StateDirectory = "nostrbox";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.dataDir ];
        PrivateTmp = true;

        Restart = "on-failure";
        RestartSec = 5;
      };

      environment = {
        NOSTRBOX_CONFIG = cfg.configFile;
        RUST_LOG = "info,nostrbox=debug";
      };
    };

    # Firewall rules
    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts = [
        (lib.toInt (lib.last (lib.splitString ":" cfg.bindAddress)))
        cfg.relayPort
      ] ++ lib.optionals cfg.fips.enable [
        (lib.toInt (lib.last (lib.splitString ":" cfg.fips.listenAddress)))
      ];
      allowedUDPPorts = lib.optionals cfg.fips.enable [
        (lib.toInt (lib.last (lib.splitString ":" cfg.fips.listenAddress)))
      ];
    };
  };
}
