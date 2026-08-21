{ builderKit, lib, ... }:
{
  home.username = builderKit.user.username;
  home.homeDirectory = builderKit.user.homeDirectory;
  home.stateVersion = "26.11";

  programs = {
    home-manager.enable = true;

    zsh = {
      enable = true;
      autosuggestion.enable = true;
      syntaxHighlighting.enable = true;
      history.size = 10000;

      # PROGRAMS ZSH BODY IS EMBEDDED HERE.
    };
  };
}
