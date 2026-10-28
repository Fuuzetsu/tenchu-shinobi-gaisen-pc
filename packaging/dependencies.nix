{
  pkgs,
  nixpkgsSource,
  cargoDeps,
  source,
}:
let
  inherit (pkgs) lib;
  # These packages supply copyleft library binaries in the portable Linux bundle.
  # Package-wide licenses also cover tools we do not distribute (FLAC, MinGW).
  copyleftLibraries =
    packages:
    with packages;
    lib.optionals stdenv.isLinux [
      stdenv.cc.libc
      stdenv.cc.cc
      alsa-lib
      libpulseaudio
      systemdLibs
      systemdMinimal
      dbus
      glib
      libsndfile
      lame
      libmpg123
    ];
  sourceLibraries = copyleftLibraries pkgs;
  sourcePackages =
    map
      (
        group:
        let
          first = builtins.head group;
        in
        assert builtins.all (p: p.src == first.src) group;
        first // { patches = lib.unique (lib.concatMap (p: p.patches or [ ]) group); }
      )
      (
        builtins.attrValues (
          lib.groupBy (p: p.name) (
            lib.concatMap (system: copyleftLibraries (import nixpkgsSource { inherit system; })) [
              "x86_64-linux"
              "aarch64-linux"
            ]
          )
        )
      );
  libraries =
    sourceLibraries
    ++ (
      with pkgs;
      lib.optionals stdenv.isLinux [
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
        libffi
        pcre2
        libxml2
        flac
        libogg
        libvorbis
        libopus
        xkeyboard_config
        pkgsCross.mingwW64.windows.mingw_w64
      ]
    );
  components = libraries ++ [ pkgs.rustc-unwrapped ];
  sourceNames = map (p: p.name) sourceLibraries;
  sourceKey = p: builtins.unsafeDiscardStringContext (toString p.src);
  sourceGroups = lib.groupBy sourceKey sourcePackages;
  sourceArchive = p: "upstream/${(builtins.head sourceGroups.${sourceKey p}).name}";
  recipe =
    p: lib.removePrefix "${nixpkgsSource}/" (builtins.head (lib.splitString ":" p.meta.position));
  nixpkgsRoot = /. + builtins.unsafeDiscardStringContext (toString nixpkgsSource);
  recipes = lib.fileset.toSource {
    root = nixpkgsRoot;
    fileset = lib.fileset.unions (
      [
        (nixpkgsRoot + "/COPYING")
        (nixpkgsRoot + "/lib")
        (nixpkgsRoot + "/pkgs/stdenv")
        (nixpkgsRoot + "/pkgs/build-support")
      ]
      ++ map (p: builtins.dirOf (nixpkgsRoot + "/${recipe p}")) sourcePackages
    );
  };
  sourceManifest = pkgs.writeText "source-packages.json" (
    builtins.toJSON {
      nixpkgsRevision = nixpkgsSource.rev;
      components = map (p: {
        inherit (p) name;
        source = sourceArchive p;
        recipe = "recipes/${recipe p}";
        patches = map (patch: "patches/${p.name}/${builtins.baseNameOf (toString patch)}") (
          p.patches or [ ]
        );
      }) sourcePackages;
    }
  );
  manifest = pkgs.writeText "native-dependencies.json" (
    builtins.toJSON (
      map (p: {
        name = p.name;
        source = toString p.src;
        patches = map toString (p.patches or [ ]);
        outputs = map (o: builtins.unsafeDiscardStringContext (toString p.${o})) p.outputs;
        licenses = map (l: l.spdxId or l.shortName) (lib.toList p.meta.license);
        source_archive = if builtins.elem p.name sourceNames then sourceArchive p else null;
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
    mkdir -p "$out/upstream" "$out/patches" "$out/recipes"
    ${lib.concatMapStringsSep "\n" (
      group:
      let
        p = builtins.head group;
      in
      ''
        cp -R ${p.src} "$out/${sourceArchive p}"
      ''
    ) (builtins.attrValues sourceGroups)}
    ${lib.concatMapStringsSep "\n" (p: ''
      mkdir -p "$out/patches/${p.name}"
      ${lib.concatMapStringsSep "\n" (patch: ''
        cp -L ${patch} "$out/patches/${p.name}/${builtins.baseNameOf (toString patch)}"
      '') (p.patches or [ ])}
    '') sourcePackages}
    cp -R ${recipes}/. "$out/recipes/"
    cp ${sourceManifest} "$out/source-packages.json"
  '';
}
