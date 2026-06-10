{
  lib,
  pkgs,
  crane,
  ...
}:
let
  craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default);

  unfilteredRoot = ../.;

  src = lib.fileset.toSource {
    root = unfilteredRoot;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources unfilteredRoot)
      ../assets/font/FiraCode-Regular.ttf
    ];
  };

  commonArgs = {
    inherit src;
    strictDeps = true;
    __structuredAttrs = true;

    nativeBuildInputs = with pkgs; [
      pkg-config
    ];

    buildInputs = with pkgs; [
      fontconfig
      libxkbcommon.dev
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  kymenu = craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
      meta.mainProgram = "kymenu";
    }
  );

  checks = {
    inherit kymenu;
    webhook-router-clippy = craneLib.cargoClippy (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      }
    );
  };

  devShell = craneLib.devShell {
    checks = checks;
  };
in
{
  inherit checks devShell kymenu;
}
