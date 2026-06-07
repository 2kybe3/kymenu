{
  pkgs,
  crane,
  ...
}:
let
  craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default);

  src = craneLib.cleanCargoSource ../.;

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

  kybar = craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
      meta.mainProgram = "kybar";
    }
  );

  checks = {
    inherit kybar;
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
  inherit checks devShell kybar;
}
