{ pkgs, ... }:
{
  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  nixpkgs.hostPlatform = system;
  nixpkgs.config.allowUnfree = true;

  system.stateVersion = "26.05";
  system.configurationRevision = inputs.self.rev or inputs.self.dirtyRev or null;

  wsl = {
    enable = true;
    defaultUser = builderKit.user.username;

    # Required so the generated flake and adjacent lock file are copied into
    # /etc/nixos in the installed WSL image.
    tarball.configPath = ./.;
  };

  users.users.${builderKit.user.username} = {
    isNormalUser = true;
    description = builderKit.user.fullName;
    home = builderKit.user.homeDirectory;
    shell = pkgs.zsh;
    extraGroups = [ "wheel" ];
  };

  programs.zsh.enable = true;

  security.sudo.wheelNeedsPassword = false;

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
