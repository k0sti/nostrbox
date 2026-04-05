{ config, lib, pkgs, ... }:

let
  cfg = config.services.nostr-relay;

  strfryConfig = pkgs.writeText "strfry.conf" ''
    db = "${cfg.dataDir}/strfry-db/"

    relay {
        bind = "${cfg.bind}"
        port = ${toString cfg.port}
        nofiles = 524288

        info {
            name = "${cfg.name}"
            description = "${cfg.description}"
        }
    }
  '';
in {
  options.services.nostr-relay = {
    enable = lib.mkEnableOption "Strfry Nostr relay";

    bind = lib.mkOption {
      type = lib.types.str;
      default = "::";
      description = "Address to bind to";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 4869;
      description = "WebSocket port";
    };

    name = lib.mkOption {
      type = lib.types.str;
      default = "Nostr Relay";
      description = "Relay name shown in NIP-11 info";
    };

    description = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = "Relay description shown in NIP-11 info";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/strfry";
      description = "Data directory for strfry database";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall for relay port";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.strfry = {
      description = "Strfry Nostr Relay";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "strfry";
        Group = "strfry";
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${cfg.dataDir}/strfry-db";
        ExecStart = "${pkgs.strfry}/bin/strfry --config=${strfryConfig} relay";
        WorkingDirectory = cfg.dataDir;
        Restart = "on-failure";
        RestartSec = 10;
        StateDirectory = "strfry";

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.dataDir ];
      };
    };

    users.users.strfry = {
      isSystemUser = true;
      group = "strfry";
      home = cfg.dataDir;
    };
    users.groups.strfry = {};

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
