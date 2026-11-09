{
  lib,
  rustPlatform,
  pkg-config,
}:

let
  cargoToml = lib.importTOML ../../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "lightforge";
  inherit (cargoToml.workspace.package) version;

  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../Cargo.toml
      ../../Cargo.lock
      ../../crates
    ];
  };

  cargoLock.lockFile = ../../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];

  meta = {
    description = "Install, update and play World of Warcraft and the other Battle.net games on Linux, from the command line";
    homepage = cargoToml.workspace.package.homepage;
    license = lib.licenses.gpl3Plus;
    mainProgram = "lightforge";
    platforms = [ "x86_64-linux" ];
  };
}
