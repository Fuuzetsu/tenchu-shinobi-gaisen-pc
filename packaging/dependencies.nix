{
  pkgs,
  nixpkgsSource,
  cargoDeps,
  source,
}:
let
  inherit (pkgs) lib;
  libraries =
    with pkgs;
    lib.optionals stdenv.isLinux [
      stdenv.cc.libc
      stdenv.cc.cc
      alsa-lib
      libpulseaudio
      systemdLibs
      systemdMinimal
      vulkan-loader
      libglvnd
      wayland
      libxkbcommon
      libX11
      libXcursor
      libXi
      libXrandr
      libXau
      libXdmcp
      libXext
      libXfixes
      libXrender
      libxcb
      dbus
      glib
      libffi
      pcre2
      libxml2
      libsndfile
      flac
      libogg
      libvorbis
      libopus
      lame
      libmpg123
      xkeyboard_config
      pkgsCross.mingwW64.windows.mingw_w64
    ];
  components = libraries ++ [ pkgs.rustc-unwrapped ];
  manifest = pkgs.writeText "native-dependencies.json" (
    builtins.toJSON (
      map (p: {
        name = p.name;
        source = toString p.src;
        patches = map toString (p.patches or [ ]);
        outputs = map (o: builtins.unsafeDiscardStringContext (toString p.${o})) p.outputs;
        licenses = map (l: l.spdxId or l.shortName) (lib.toList p.meta.license);
      }) components
    )
  );
in
{
  inherit manifest;
  notices = pkgs.stdenvNoCC.mkDerivation {
    name = "tenchu-dependency-notices";
    src = source;
    inherit cargoDeps;
    nativeBuildInputs = [
      pkgs.cargo
      pkgs.cargo-about
      pkgs.rustPlatform.cargoSetupHook
      pkgs.python3
    ];
    dontConfigure = true;
    buildPhase = ''
      mkdir notices
      cargo-about generate --frozen --workspace --fail --config ${./about.toml} \
        ${./about.hbs} -o notices/rust.html
      python3 ${./dependency-notices.py} ${manifest} notices/native
    '';
    installPhase = ''
      mkdir -p "$out"
      cp -R notices/. "$out/"
      cp ${manifest} "$out/native-dependencies.json"
    '';
  };
  sources = pkgs.runCommand "tenchu-dependency-sources" { } ''
    mkdir -p "$out/upstream" "$out/patches"
    ${lib.concatMapStringsSep "\n" (p: ''
      cp -R ${p.src} "$out/upstream/${p.name}"
      mkdir -p "$out/patches/${p.name}"
      ${lib.concatMapStringsSep "\n" (patch: ''
        cp -L ${patch} "$out/patches/${p.name}/${builtins.baseNameOf (toString patch)}"
      '') (p.patches or [ ])}
    '') components}
    cp -RL ${cargoDeps} "$out/rust-crates"
    cp -R ${nixpkgsSource} "$out/nixpkgs"
    cp ${manifest} "$out/native-dependencies.json"
  '';
}
