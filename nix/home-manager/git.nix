{ builderKit, ... }:
{
  programs.git = {
    enable = true;
    userName = builderKit.user.fullName;
    userEmail = builderKit.user.email;
    extraConfig.init.defaultBranch = builderKit.git.defaultBranch;
  };
}
