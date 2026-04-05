{
  description = "NostrBox NixOS machine configurations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fips = {
      url = "github:k0sti/fips";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fips, ... }:
    let
      pkgsFor = system: import nixpkgs { inherit system; };
      specialArgs = { inputs = { inherit self fips; }; };
      overlay = final: prev: {
        blossom-server = final.callPackage ./packages/blossom-server.nix {};
      };
    in {
      # Machine configurations
      # Build: nixos-rebuild switch --flake .#mac-mini
      nixosConfigurations.mac-mini = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        inherit specialArgs;
        modules = [
          { nixpkgs.overlays = [ overlay ]; }
          ./modules/nostrbox.nix
          ./modules/fips.nix
          ./users.nix
          ./machines/mac-mini/configuration.nix
          ./profiles/appliance.nix
        ];
      };

      nixosConfigurations.mac-mini-dev = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        inherit specialArgs;
        modules = [
          { nixpkgs.overlays = [ overlay ]; }
          ./modules/nostrbox.nix
          ./modules/fips.nix
          ./users.nix
          ./machines/mac-mini/configuration.nix
          ./profiles/dev.nix
        ];
      };
    };
}
