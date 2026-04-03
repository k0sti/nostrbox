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
    storage:
      backend: local
      local:
        dir: ./data/blobs

    server:
      port: 24242
      host: "::"

    rules:
      - type: all
        expiration: false
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
      ExecStart = "${pkgs.nodejs_22}/bin/npx blossom-server-ts";
      WorkingDirectory = "/var/lib/blossom";
      Restart = "on-failure";
      RestartSec = 10;
      StateDirectory = "blossom";

      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ReadWritePaths = [ "/var/lib/blossom" ];
    };

    environment = {
      HOME = "/var/lib/blossom";
      npm_config_cache = "/var/lib/blossom/.npm";
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
    "d /var/lib/blossom/data 0755 blossom blossom -"
    "L /var/lib/blossom/config.yml - - - - ${blossomConfig}"
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
    nodejs_22  # for blossom npx
    inputs.fips.packages.x86_64-linux.fips-ble  # fipsctl, fipstop
  ];

  # ---------- Misc ----------
  time.timeZone = "Atlantic/Madeira";
  system.stateVersion = "25.11";
}
