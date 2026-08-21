{
  description = "Builder Kit generated macOS development environment";

  inputs = {
{{ flake_inputs }}
  };

  outputs = inputs@{ self, nixpkgs, nix-darwin, home-manager }:
    let
      # BEGIN BUILDER-KIT USER SETTINGS
      builderKit = {
        settingsSchemaVersion = 1;
        hostName = "builderkit-mac";

        user = {
          username = "student";
          homeDirectory = "/Users/student";
          fullName = "Builder Kit Student";
          email = "student@example.com";
        };

        git.defaultBranch = "main";
      };
      # END BUILDER-KIT USER SETTINGS

      supportedSystems = [
        "aarch64-darwin"
      ];

      forEachSystem = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      # BEGIN BUILDER-KIT MANAGED IMPLEMENTATION
      darwinConfigurations = forEachSystem (system:
{{ darwin_configuration }}
      );
      # END BUILDER-KIT MANAGED IMPLEMENTATION
    };
}
