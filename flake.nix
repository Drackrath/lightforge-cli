{
  description = "Lightforge: a command-line launcher for World of Warcraft and Battle.net on Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" ];
      forAllSystems = f: lib.genAttrs systems (system: f system nixpkgs.legacyPackages.${system});
    in
    {
      overlays.default = final: _prev: {
        lightforge-unwrapped = final.callPackage ./packaging/nix/package.nix { };
        lightforge = final.callPackage ./packaging/nix/fhs.nix { };
      };

      packages = forAllSystems (_: pkgs: rec {
        lightforge-unwrapped = pkgs.callPackage ./packaging/nix/package.nix { };
        lightforge = pkgs.callPackage ./packaging/nix/fhs.nix { inherit lightforge-unwrapped; };
        default = lightforge;
      });

      devShells = forAllSystems (system: pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.lightforge-unwrapped ];
          packages = [ pkgs.clippy pkgs.rustfmt pkgs.rust-analyzer ];
        };
      });
    };
}
