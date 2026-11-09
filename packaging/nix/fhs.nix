{
  symlinkJoin,
  makeWrapper,
  steam-run-free,
  lightforge-unwrapped,
}:

# Proton's prebuilt binaries expect the FHS loader paths, which NixOS does not
# have (no /lib/ld-linux.so.2 even with hardware.graphics.enable32Bit), so
# Lightforge runs inside steam-run's FHS environment.
# steam-run-free is steam-run without the unfree Steam package.
symlinkJoin {
  name = "lightforge-${lightforge-unwrapped.version}";
  paths = [ lightforge-unwrapped ];
  nativeBuildInputs = [ makeWrapper ];

  postBuild = ''
    rm $out/bin/lightforge
    makeWrapper ${steam-run-free}/bin/steam-run $out/bin/lightforge \
      --add-flags ${lightforge-unwrapped}/bin/lightforge
  '';

  inherit (lightforge-unwrapped) version meta;
}
