{
  programs.uv = {
    enable = true;
    settings = {
      python-downloads = "manual";
      python-preference = "only-managed";
    };
    python = {
      prune = true;
      versions = [ "3.14" ];
      default = [ "3.14" ];
    };
    tool = {
      prune = true;
      packages = [ ];
    };
  };
}
