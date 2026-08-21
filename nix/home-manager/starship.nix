{
  programs.starship = {
    enable = true;
    enableZshIntegration = true;

    settings = {
      format = "[│--> ](bold green)$all";

      character = {
        success_symbol = "[>](bold green)";
        error_symbol = "[x](bold red)";
        vimcmd_symbol = "[<](bold green)";
      };

      git_metrics = {
        disabled = false;
      };

      kubernetes = {
        disabled = false;
      };
    };
  };
}
