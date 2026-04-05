{ config, lib, pkgs, inputs, ... }:

let
  strfryConfig = pkgs.writeText "strfry.conf" ''
    db = "/var/lib/strfry/strfry-db/"

    relay {
        bind = "::"
        port = 4869
        nofiles = 524288

        info {
            name = "NostrBox Relay"
            description = "NostrBox appliance relay"
        }
    }
  '';

  blossomConfig = pkgs.writeText "config.yml" ''
    db_path: /var/lib/blossom/db/database.sqlite3
    api_addr: "[::]:24242"
    cdn_url: http://localhost:24242
    max_upload_size_bytes: 104857600
    allowed_mime_types:
      - "*"
  '';
in
{
  imports = [
    ./hardware-configuration.nix
  ];

  # ---------- Boot ----------
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Broadcom BCM4331 WiFi (non-free driver)
  boot.kernelModules = [ "wl" ];
  boot.extraModulePackages = [ config.boot.kernelPackages.broadcom_sta ];
  boot.blacklistedKernelModules = [ "b43" "bcma" ];
  hardware.enableAllFirmware = true;
  nixpkgs.config.allowUnfree = true;
  nixpkgs.config.permittedInsecurePackages = [
    "broadcom-sta-6.30.223.271-59-6.18.20"
  ];

  # ---------- Networking ----------
  networking.hostName = "nostrbox";
  networking.networkmanager.enable = true;

  # ---------- DNS: route .fips domains to FIPS DNS responder ----------
  services.dnsmasq = {
    enable = true;
    settings = {
      listen-address = "127.0.0.1";
      bind-interfaces = true;
      server = [
        "/fips/127.0.0.1#5354"
        "1.1.1.1"
        "8.8.8.8"
      ];
    };
  };

  # ---------- SSH ----------
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      PermitRootLogin = "no";
    };
  };

  # ---------- FIPS mesh networking ----------
  services.fips = {
    enable = true;
    package = inputs.fips.packages.x86_64-linux.fips-ble;
    configFile = "/home/k0/.config/fips/fips.yaml";
    transports = [ "udp" ];
    ethernet = {
      enable = true;
      interface = "enp3s0f0";
    };
    peers = [
      {
        npub = "npub1vu597zwwq0j9jksuptc9u4wmhavykuk44djlq7xu90pesueu3rdsnm32ah";
        alias = "zephyrus";
        addresses = [
          { transport = "udp"; addr = "10.10.243.238:2121"; }
        ];
      }
    ];
  };

  # ---------- Strfry Nostr relay ----------
  systemd.services.strfry = {
    description = "Strfry Nostr Relay";
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    wantedBy = [ "multi-user.target" ];

    serviceConfig = {
      Type = "simple";
      User = "strfry";
      Group = "strfry";
      ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p /var/lib/strfry/strfry-db";
      ExecStart = "${pkgs.strfry}/bin/strfry --config=${strfryConfig} relay";
      WorkingDirectory = "/var/lib/strfry";
      Restart = "on-failure";
      RestartSec = 10;
      StateDirectory = "strfry";

      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ReadWritePaths = [ "/var/lib/strfry" ];
    };
  };

  users.users.strfry = {
    isSystemUser = true;
    group = "strfry";
    home = "/var/lib/strfry";
  };
  users.groups.strfry = {};

  # ---------- Blossom media server ----------
  systemd.services.blossom = {
    description = "Blossom Media Server";
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    wantedBy = [ "multi-user.target" ];

    serviceConfig = {
      Type = "simple";
      User = "blossom";
      Group = "blossom";
      ExecStart = "${pkgs.blossom-server}/bin/blossom-server";
      WorkingDirectory = "/var/lib/blossom";
      Restart = "on-failure";
      RestartSec = 10;
      StateDirectory = "blossom";

      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ReadWritePaths = [ "/var/lib/blossom" ];
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
    "d /var/lib/blossom/db 0755 blossom blossom -"
    "L /var/lib/blossom/config.yml - - - - ${blossomConfig}"
    # Go blossom-server reads migrations from db/migrations relative to CWD
    "L+ /var/lib/blossom/db/migrations - - - - ${pkgs.blossom-server}/share/blossom-server/db/migrations"
  ];

  # ---------- NostrBox service ----------
  # Uncomment once the package builds
  # services.nostrbox = {
  #   enable = true;
  #   package = inputs.self.packages.x86_64-linux.default;
  #   webDistPath = "/var/lib/nostrbox/web";
  #   bindAddress = "0.0.0.0:3400";
  #   openFirewall = true;
  # };

  # ---------- Firewall ----------
  networking.firewall.allowedTCPPorts = [
    4869   # strfry relay (ws)
    24242  # blossom media server
  ];

  # ---------- Nix ----------
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # ---------- System packages ----------
  environment.systemPackages = with pkgs; [
    git
    vim
    htop
    just
    tmux
    curl
    nak
    strfry
    inputs.fips.packages.x86_64-linux.fips-ble  # fipsctl, fipstop
  ];

  # ---------- Misc ----------
  time.timeZone = "Atlantic/Madeira";
  system.stateVersion = "25.11";
}
