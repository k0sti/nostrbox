{ config, lib, pkgs, inputs, ... }:

{
  # Production appliance profile
  # Hardened defaults, FIPS enabled, firewall on

  # services.nostrbox = {
  #   enable = true;
  #   package = inputs.self.packages.x86_64-linux.default;
  #   webDistPath = "/var/lib/nostrbox/web";
  #   bindAddress = "0.0.0.0:3400";
  #   openFirewall = true;
  #   fips = {
  #     enable = true;
  #     package = inputs.fips.packages.x86_64-linux.fips-ble;
  #     transports = [ "udp" "tcp" "ble" ];
  #   };
  # };

  # Firewall defaults
  networking.firewall.enable = true;

  # Automatic upgrades (NixOS)
  # system.autoUpgrade.enable = true;
}
