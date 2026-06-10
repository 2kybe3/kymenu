{
  nixConfig.extra-substituters = [ "https://attic.kybe.xyz/main" ];
  nixConfig.extra-trusted-public-keys = [
    "main:cb7V485kGP0lG7LtQ/suOgKOgtVxNXrnD6i5yCtnaMQ="
  ];

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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
      treefmt-nix,
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

        kymenu = pkgs.callPackage ./nix/kymenu.nix { inherit self crane; };

        treefmt-eval = treefmt-nix.lib.evalModule pkgs ./nix/treefmt.nix;
      in
      {
        packages.default = kymenu.kymenu;

        checks = kymenu.checks // {
          formatting = treefmt-eval.config.build.check self;
        };

        devShells.default = kymenu.devShell;
        formatter = treefmt-eval.config.build.wrapper;
      }
    );
}
