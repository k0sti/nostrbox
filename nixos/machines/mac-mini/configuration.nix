{ config, lib, pkgs, inputs, ... }:

{
  imports = [
    ./hardware-configuration.nix
  ];

  # ---------- Graphics (Intel integrated) ----------
  hardware.graphics = {
    enable = true;
    extraPackages = with pkgs; [
      intel-vaapi-driver      # i965 VA-API (Haswell)
      libva-vdpau-driver
      libvdpau-va-gl
    ];
  };

  # Force i965 driver for Haswell (iHD doesn't support pre-Broadwell)
  environment.variables.LIBVA_DRIVER_NAME = "i965";

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
    configFile = "/etc/fips/fips.yaml";
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

  # ---------- Nostr relay ----------
  services.nostr-relay = {
    enable = true;
    name = "NostrBox Relay";
    description = "NostrBox appliance relay";
    openFirewall = true;
  };

  # ---------- Blossom media server (route96) ----------
  services.blossom = {
    enable = true;
    listen = "[::]:24242";
    publicUrl = "http://localhost:24242";
    maxUploadBytes = 524288000; # 500MB
    openFirewall = true;
  };

  # ---------- NostrBox service ----------
  # Uncomment once the package builds
  # services.nostrbox = {
  #   enable = true;
  #   package = inputs.self.packages.x86_64-linux.default;
  #   webDistPath = "/var/lib/nostrbox/web";
  #   bindAddress = "0.0.0.0:3400";
  #   openFirewall = true;
  # };

  # ---------- Power management (never suspend — services must stay up) ----------
  powerManagement.enable = false;
  systemd.targets.sleep.enable = false;
  systemd.targets.suspend.enable = false;
  systemd.targets.hibernate.enable = false;
  systemd.targets.hybrid-sleep.enable = false;
  services.logind.settings.Login.IdleAction = "ignore";

  # ---------- Firewall ----------
  networking.firewall.trustedInterfaces = [ "fips0" ];

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
