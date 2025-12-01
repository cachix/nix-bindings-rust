{
  pkgs,
  lib,
  config,
  ...
}:
{
  languages.rust.enable = lib.mkForce true;
  packages = [
    pkgs.protobuf
    pkgs.pkg-config
    pkgs.nix
  ];
}
