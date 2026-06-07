{
  nixConfig.extra-substituters = [ "https://attic.kybe.xyz/main" ];
  nixConfig.extra-trusted-public-keys = [
    "main:cb7V485kGP0lG7LtQ/suOgKOgtVxNXrnD6i5yCtnaMQ="
  ];

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      crane,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        kybar = pkgs.callPackage ./nix/kybar.nix { inherit self crane; };
      in
      {
        packages.default = kybar.kybar;

        inherit (kybar) checks;

        devShells.default = kybar.devShell;
      }
    );
}
