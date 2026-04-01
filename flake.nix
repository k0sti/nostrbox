{
  description = "NostrBox — sovereign Nostr community server appliance";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";

    # FIPS mesh networking
    # Local dev: nix flake update fips --override-input fips path:../fips
    # Prod: pinned to git commit
    fips = {
      url = "github:k0sti/fips";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, fips, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Native deps needed by nostrbox and its dependencies
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          just
        ];

        buildInputs = with pkgs; [
          openssl
          sqlite
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        # FIPS packages (BLE variant on Linux, base on macOS)
        fipsPkgs = fips.packages.${system};
        fipsPackage = if pkgs.stdenv.isLinux then fipsPkgs.fips-ble else fipsPkgs.fips;

        # Web UI deps
        webInputs = with pkgs; [
          bun
          nodejs_22
        ];

      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeBuildInputs ++ webInputs ++ [ fipsPackage ];
          inherit buildInputs;

          shellHook = ''
            echo "🔲 NostrBox dev shell"
            echo "   rust: $(rustc --version)"
            echo "   fips: ${fipsPackage.name} (${if pkgs.stdenv.isLinux then "BLE enabled" else "no BLE — macOS"})"
            echo "   cargo build / just run / just dev"
          '';

          # For openssl-sys
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          # For rusqlite bundled feature (uses cc)
          # If switching to system sqlite, set SQLITE3_LIB_DIR instead
        };

        packages.fips = fipsPackage;
        packages.fips-base = fipsPkgs.fips;

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "nostrbox";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = buildInputs;

          # contextvm-sdk is a path dep outside the repo — override for CI builds
          # For local dev, cargo build in the dev shell works fine
          meta.broken = true; # enable once contextvm-sdk is vendored or published
        };
      }
    ) // {
      # NixOS module for appliance deployment
      nixosModules.fips = import ./nixos/modules/fips.nix;

      nixosModules.nostrbox = { config, lib, pkgs, ... }:
        let
          cfg = config.services.nostrbox;
        in {
          options.services.nostrbox = {
            enable = lib.mkEnableOption "NostrBox appliance";

            package = lib.mkOption {
              type = lib.types.package;
              description = "NostrBox server package";
            };

            dataDir = lib.mkOption {
              type = lib.types.path;
              default = "/var/lib/nostrbox";
              description = "Data directory for NostrBox state";
            };

            configFile = lib.mkOption {
              type = lib.types.path;
              default = "/etc/nostrbox/nostrbox.toml";
              description = "Path to NostrBox config file";
            };

            webDistPath = lib.mkOption {
              type = lib.types.path;
              description = "Path to built web UI dist";
            };

            bindAddress = lib.mkOption {
              type = lib.types.str;
              default = "0.0.0.0:3400";
              description = "Address:port for the management UI/API";
            };

            relayPort = lib.mkOption {
              type = lib.types.port;
              default = 7777;
              description = "Nostr relay WebSocket port";
            };

            openFirewall = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Open firewall for relay and management ports";
            };

            fips = {
              enable = lib.mkEnableOption "FIPS mesh networking";

              package = lib.mkOption {
                type = lib.types.package;
                description = "FIPS package (use fips-ble for BLE support)";
              };

              listenAddress = lib.mkOption {
                type = lib.types.str;
                default = "0.0.0.0:9735";
                description = "FIPS listen address";
              };

              transports = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [ "udp" "tcp" ];
                description = "Enabled FIPS transports";
              };

              peers = lib.mkOption {
                type = lib.types.listOf lib.types.str;
                default = [];
                description = "Static FIPS peers (npub@host:port)";
              };

              socketPath = lib.mkOption {
                type = lib.types.str;
                default = "/run/fips/fips.sock";
                description = "FIPS control socket path";
              };
            };
          };

          config = lib.mkIf cfg.enable {
            users.users.nostrbox = {
              isSystemUser = true;
              group = "nostrbox";
              home = cfg.dataDir;
              createHome = true;
            };
            users.groups.nostrbox = {};

            systemd.services.nostrbox = {
              description = "NostrBox Server";
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];

              serviceConfig = {
                Type = "simple";
                User = "nostrbox";
                Group = "nostrbox";
                ExecStart = "${cfg.package}/bin/nostrbox-server";
                WorkingDirectory = cfg.dataDir;
                StateDirectory = "nostrbox";

                # Hardening
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                ReadWritePaths = [ cfg.dataDir ];
                PrivateTmp = true;

                Restart = "on-failure";
                RestartSec = 5;
              };

              environment = {
                NOSTRBOX_CONFIG = cfg.configFile;
                RUST_LOG = "info,nostrbox=debug";
              };
            };

            # Firewall rules
            networking.firewall = lib.mkIf cfg.openFirewall {
              allowedTCPPorts = [
                (lib.toInt (lib.last (lib.splitString ":" cfg.bindAddress)))
                cfg.relayPort
              ] ++ lib.optionals cfg.fips.enable [
                (lib.toInt (lib.last (lib.splitString ":" cfg.fips.listenAddress)))
              ];
              allowedUDPPorts = lib.optionals cfg.fips.enable [
                (lib.toInt (lib.last (lib.splitString ":" cfg.fips.listenAddress)))
              ];
            };
          };
        };

      # Machine configurations
      # Build: nixos-rebuild switch --flake .#mac-mini
      nixosConfigurations.mac-mini = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inputs = { inherit self; inherit fips; }; };
        modules = [
          self.nixosModules.nostrbox
          self.nixosModules.fips
          ./nixos/machines/mac-mini/configuration.nix
          ./nixos/profiles/appliance.nix
        ];
      };

      nixosConfigurations.mac-mini-dev = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inputs = { inherit self; inherit fips; }; };
        modules = [
          self.nixosModules.nostrbox
          self.nixosModules.fips
          ./nixos/machines/mac-mini/configuration.nix
          ./nixos/profiles/dev.nix
        ];
      };
    };
}
