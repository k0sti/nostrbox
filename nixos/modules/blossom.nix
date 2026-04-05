{ config, lib, pkgs, ... }:

let
  cfg = config.services.blossom;

  configFile = pkgs.writeText "config.yaml" ''
    listen: "${cfg.listen}"
    database: "${cfg.database}"
    storage_dir: "${cfg.storageDir}"
    max_upload_bytes: ${toString cfg.maxUploadBytes}
    public_url: "${cfg.publicUrl}"
  '';
in {
  options.services.blossom = {
    enable = lib.mkEnableOption "route96 Blossom/NIP-96 media server";

    listen = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0:24242";
      description = "Listen address";
    };

    database = lib.mkOption {
      type = lib.types.str;
      default = "mysql://blossom:blossom@localhost/route96?socket=/run/mysqld/mysqld.sock";
      description = "MySQL connection string";
    };

    storageDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/blossom/data";
      description = "Directory for stored blobs";
    };

    maxUploadBytes = lib.mkOption {
      type = lib.types.int;
      default = 524288000; # 500MB
      description = "Maximum upload size in bytes";
    };

    publicUrl = lib.mkOption {
      type = lib.types.str;
      default = "http://localhost:24242";
      description = "Public-facing URL for blob URLs in responses";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall for blossom port";
    };
  };

  config = lib.mkIf cfg.enable {
    # MariaDB for route96
    services.mysql = {
      enable = true;
      package = pkgs.mariadb;
      ensureDatabases = [ "route96" ];
      ensureUsers = [
        {
          name = "blossom";
          ensurePermissions = {
            "route96.*" = "ALL PRIVILEGES";
          };
        }
      ];
      # Switch blossom user from unix_socket to native password auth (sqlx doesn't support unix_socket)
      initialScript = pkgs.writeText "blossom-mysql-init.sql" ''
        ALTER USER IF EXISTS 'blossom'@'localhost' IDENTIFIED BY 'blossom';
        FLUSH PRIVILEGES;
      '';
    };

    systemd.services.blossom = {
      description = "route96 Blossom/NIP-96 Media Server";
      after = [ "network-online.target" "mysql.service" ];
      wants = [ "network-online.target" ];
      requires = [ "mysql.service" ];
      wantedBy = [ "multi-user.target" ];

      environment.RUST_LOG = "info,route96=debug";

      serviceConfig = {
        Type = "simple";
        User = "blossom";
        Group = "blossom";
        ExecStart = "${pkgs.route96}/bin/route96 --config ${configFile}";
        WorkingDirectory = "/var/lib/blossom";
        Restart = "on-failure";
        RestartSec = 10;
        StateDirectory = "blossom";

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/var/lib/blossom" cfg.storageDir ];
      };
    };

    users.users.blossom = {
      isSystemUser = true;
      group = "blossom";
      home = "/var/lib/blossom";
    };
    users.groups.blossom = {};

    systemd.tmpfiles.rules = [
      "d /var/lib/blossom 0755 blossom blossom -"
      "d ${cfg.storageDir} 0755 blossom blossom -"
    ];

    networking.firewall.allowedTCPPorts =
      let port = lib.toInt (lib.last (lib.splitString ":" cfg.listen));
      in lib.mkIf cfg.openFirewall [ port ];
  };
}
