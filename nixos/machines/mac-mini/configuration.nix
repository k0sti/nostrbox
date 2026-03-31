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
  hardware.enableAllFirmware = true;
  nixpkgs.config.allowUnfree = true;

  # ---------- Networking ----------
  networking.hostName = "nostrbox";
  networking.networkmanager.enable = true;

  # ---------- Swap (4GB RAM is tight for Rust builds) ----------
  swapDevices = [{
    device = "/var/lib/swapfile";
    size = 8192; # MB
  }];

  # ---------- SSH ----------
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      PermitRootLogin = "no";
    };
  };

  # ---------- User ----------
  users.users.k0 = {
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" ];
    openssh.authorizedKeys.keys = [
      # TODO: add your SSH pubkey here
      # "ssh-ed25519 AAAA..."
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
  #   fips = {
  #     enable = true;
  #     transports = [ "udp" "tcp" ];
  #   };
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
  ];

  # ---------- Misc ----------
  time.timeZone = "Europe/Helsinki";
  system.stateVersion = "25.05";
}
