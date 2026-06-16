{ pkgs, ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    taplo.enable = true;
    typos.enable = true;
    nixfmt.enable = true;
    rustfmt = {
      enable = true;
      package = pkgs.rust-bin.stable.latest.default;
    };
  };
  settings = {
    excludes = [
      "target/*"
      "result/*"
      ".git/*"
    ];
  };

}
