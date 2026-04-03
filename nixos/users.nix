{ config, lib, pkgs, ... }:

{
  users.users = {
    k0 = {
      isNormalUser = true;
      description = "k0";
      extraGroups = [ "wheel" "networkmanager" ];
      openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHpHNhhKTNylsjkd8pTNqRqe70fSnKCZINTmZ4AMnDXq k0@studio"
      ];
    };

    nostrbox = {
      isNormalUser = true;
      description = "nostrbox";
      extraGroups = [ "networkmanager" "wheel" ];
    };
  };
}
