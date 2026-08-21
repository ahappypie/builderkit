inputs.nix-darwin.lib.darwinSystem {
  inherit system;

  specialArgs = {
    inherit builderKit inputs system;
  };

  modules = [
    inputs.home-manager.darwinModules.home-manager
    # DARWIN MODULE IS EMBEDDED HERE.
  ];
}