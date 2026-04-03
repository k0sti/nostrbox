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
  time.timeZone = "Atlantic/Madeira";
  system.stateVersion = "25.11";
}
