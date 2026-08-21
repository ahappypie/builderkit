{ pkgs, ... }:
{
  home.packages = with pkgs; [
    ccusage
  ];
}
