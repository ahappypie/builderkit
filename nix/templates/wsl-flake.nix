{
  description = "Builder Kit generated WSL development environment";

  inputs = {
{{ flake_inputs }}
  };

  outputs = inputs@{ self, nixpkgs, nixos-wsl, home-manager }:
    let
      # BEGIN BUILDER-KIT USER SETTINGS
      builderKit = {
        settingsSchemaVersion = 1;
        hostName = "builderkit-wsl";

        user = {
          username = "student";
          homeDirectory = "/home/student";
          fullName = "Builder Kit Student";
          email = "student@example.com";
        };

        git.defaultBranch = "main";
      };
      # END BUILDER-KIT USER SETTINGS

      supportedSystems = [
        "x86_64-linux"
      ];

      forEachSystem = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      # BEGIN BUILDER-KIT MANAGED IMPLEMENTATION
      nixosConfigurations = forEachSystem (system:
{{ wsl_configuration }}
      );
      # END BUILDER-KIT MANAGED IMPLEMENTATION
    };
}
