{
  lib,
  rustPlatform,
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

  meta = {
    description = "Command-line launcher for World of Warcraft and the other Blizzard games on Linux";
    homepage = cargoToml.workspace.package.homepage;
    license = lib.licenses.gpl3Plus;
    mainProgram = "lightforge";
    platforms = [ "x86_64-linux" ];
  };
}
