{
  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  nixpkgs.hostPlatform = system;
  nixpkgs.config.allowUnfree = true;

  system.primaryUser = builderKit.user.username;
  system.stateVersion = 6;
  system.configurationRevision = inputs.self.rev or inputs.self.dirtyRev or null;

  users.users.${builderKit.user.username} = {
    name = builderKit.user.username;
    home = builderKit.user.homeDirectory;
  };

  programs.zsh.enable = true;

  # Builder Kit does not manage inbound SSH access by default. Students can
  # generate GitHub/user SSH keys as an explicit runbook step.
  services.openssh = {
    enable = false;
    hostKeys = [ ];
  };

  home-manager.useGlobalPkgs = true;
  home-manager.useUserPackages = true;
  home-manager.backupFileExtension = "builderkit-backup";
  home-manager.extraSpecialArgs = {
    inherit builderKit inputs;
  };

  home-manager.users.${builderKit.user.username} = {
    # HOME MANAGER MODULE BODY IS EMBEDDED HERE.
  };
}
