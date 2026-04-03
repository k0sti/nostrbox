{ config, lib, pkgs, inputs, ... }:

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

  # ---------- NostrBox service ----------
  # Uncomment once the package builds
  # services.nostrbox = {
  #   enable = true;
  #   package = inputs.self.packages.x86_64-linux.default;
  #   webDistPath = "/var/lib/nostrbox/web";
  #   bindAddress = "0.0.0.0:3400";
  #   openFirewall = true;
  # };

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
    inputs.fips.packages.x86_64-linux.fips-ble  # fipsctl, fipstop
  ];

  # ---------- Misc ----------
  time.timeZone = "Atlantic/Madeira";
  system.stateVersion = "25.11";
}
