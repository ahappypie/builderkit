nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

nix-darwin = {
  url = "github:nix-darwin/nix-darwin/master";
  inputs.nixpkgs.follows = "nixpkgs";
};

home-manager = {
  url = "github:nix-community/home-manager/master";
  inputs.nixpkgs.follows = "nixpkgs";
};
