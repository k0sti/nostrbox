{ config, lib, pkgs, inputs, ... }:

{
  # Development profile
  # Desktop environment + extra tools + relaxed firewall

  # ---------- Desktop (KDE Plasma 6) ----------
  services.xserver.enable = true;
  services.displayManager.sddm.enable = true;
  services.displayManager.defaultSession = "plasmax11";
  services.desktopManager.plasma6.enable = true;

  services.xserver.xkb = {
    layout = "fi";
    variant = "";
  };
  console.keyMap = "fi";

  # ---------- Audio (PipeWire) ----------
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  # ---------- Printing ----------
  services.printing.enable = true;

  # ---------- Locale ----------
  i18n.defaultLocale = "en_US.UTF-8";
  i18n.extraLocaleSettings = {
    LC_ADDRESS = "pt_PT.UTF-8";
    LC_IDENTIFICATION = "pt_PT.UTF-8";
    LC_MEASUREMENT = "pt_PT.UTF-8";
    LC_MONETARY = "pt_PT.UTF-8";
    LC_NAME = "pt_PT.UTF-8";
    LC_NUMERIC = "pt_PT.UTF-8";
    LC_PAPER = "pt_PT.UTF-8";
    LC_TELEPHONE = "pt_PT.UTF-8";
    LC_TIME = "pt_PT.UTF-8";
  };

  # ---------- Browser ----------
  programs.firefox.enable = true;

  # ---------- Dev packages ----------
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

    # Hardware / system debugging
    pciutils
    mesa-demos

    # General dev
    ripgrep
    fd
    bat
    jq

    # Dev tools
    bun
    nodejs
    gh
    zed-editor
    direnv
    brave
    unzip
    ioquake3
  ];

  environment.extraInit = ''
    export PATH="/home/k0/.bun/bin:$PATH"
  '';

  # ---------- Firewall (relaxed for dev) ----------
  networking.firewall.enable = true;
  networking.firewall.allowedTCPPortRanges = [
    { from = 3000; to = 3999; }  # dev servers
    { from = 7000; to = 7999; }  # relay dev
  ];
  networking.firewall.allowedUDPPorts = [
    9735  # FIPS
  ];
}
