inputs.nixpkgs.lib.nixosSystem {
  inherit system;

  specialArgs = {
    inherit builderKit inputs system;
  };

  modules = [
    inputs.nixos-wsl.nixosModules.default
    inputs.home-manager.nixosModules.home-manager
    # WSL MODULE IS EMBEDDED HERE.
  ];
}
