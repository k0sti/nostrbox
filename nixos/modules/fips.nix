{ config, lib, pkgs, ... }:

let
  cfg = config.services.nostrbox.fips;
  nostrboxCfg = config.services.nostrbox;
in {
  # Options are defined in the main nostrbox module (services.nostrbox.fips.*)
  # This module only provides the systemd service implementation.

  config = lib.mkIf (nostrboxCfg.enable && cfg.enable) {
    # FIPS config file generated from NostrBox settings
    environment.etc."fips/fips.yaml" = {
      text = builtins.toJSON {
        node = {
          identity = {
            persistent = true;
            key_dir = "${nostrboxCfg.dataDir}";
          };
          transports = builtins.listToAttrs (map (t: {
            name = t;
            value = { enabled = true; };
          }) cfg.transports);
        };
        transport = {
          udp = lib.mkIf (builtins.elem "udp" cfg.transports) {
            listen = cfg.listenAddress;
          };
          tcp = lib.mkIf (builtins.elem "tcp" cfg.transports) {
            listen = cfg.listenAddress;
          };
        };
        peers = map (p: { address = p; }) cfg.peers;
        control = {
          socket = cfg.socketPath;
        };
      };
      mode = "0644";
    };

    # FIPS systemd service
    systemd.services.fips = {
      description = "FIPS Mesh Network Daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" "nostrbox.service" ];
      wants = [ "network-online.target" ];
      requires = [ "nostrbox.service" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/fips -c /etc/fips/fips.yaml";

        # FIPS needs CAP_NET_ADMIN for TUN interface creation
        AmbientCapabilities = [ "CAP_NET_ADMIN" ];
        CapabilityBoundingSet = [ "CAP_NET_ADMIN" ];

        # Run as root but with restricted permissions
        # TUN interface creation requires root or CAP_NET_ADMIN
        DynamicUser = false;

        # Control socket directory — writable by nostrbox group for fipsctl access
        RuntimeDirectory = "fips";
        RuntimeDirectoryMode = "0750";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        ReadWritePaths = [
          nostrboxCfg.dataDir
          "/run/fips"
        ];

        Restart = "on-failure";
        RestartSec = 5;
      };

      environment = {
        RUST_LOG = "info,fips=debug";
      };
    };

    # Ensure the control socket directory has correct group ownership
    # so the nostrbox user can query via fipsctl
    systemd.tmpfiles.rules = [
      "d /run/fips 0750 root nostrbox - -"
    ];
  };
}
