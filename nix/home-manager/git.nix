{ builderKit, ... }:
{
  programs.git = {
    enable = true;
    settings = {
      user = {
        name = builderKit.user.fullName;
        email = builderKit.user.email;
      };

      init.defaultBranch = builderKit.git.defaultBranch;
    };
  };
}
