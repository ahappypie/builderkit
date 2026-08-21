nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

nixos-wsl = {
  url = "github:nix-community/NixOS-WSL/main";
  inputs.nixpkgs.follows = "nixpkgs";
};

home-manager = {
  url = "github:nix-community/home-manager/master";
  inputs.nixpkgs.follows = "nixpkgs";
};
