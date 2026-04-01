{ config, lib, pkgs, inputs, ... }:

{
  # Development profile
  # Extra tools, debug logging, relaxed settings

  environment.systemPackages = with pkgs; [
    # Rust dev
    rust-analyzer
    cargo-watch

    # FIPS debugging
    inputs.fips.packages.x86_64-linux.fips-ble

    # Network debugging
    tcpdump
    nmap
    iproute2
    wireguard-tools

    # General dev
    ripgrep
    fd
    bat
    jq
  ];

  # More permissive firewall for dev
  networking.firewall.enable = true;
  networking.firewall.allowedTCPPortRanges = [
    { from = 3000; to = 3999; }  # dev servers
    { from = 7000; to = 7999; }  # relay dev
  ];
  networking.firewall.allowedUDPPorts = [
    9735  # FIPS
  ];

  # Debug-level logging
  # services.nostrbox ... (uncomment when service works)
}
