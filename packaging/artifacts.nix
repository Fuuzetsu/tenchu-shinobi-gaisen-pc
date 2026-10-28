{
  pkgs,
  native,
  web,
  windows ? null,
  runtimeLibraries,
  revision,
  nixpkgsRevision,
  version,
  dependencies,
  portSource,
}:
let
  inherit (pkgs) lib;
  archive =
    name: contents: instructions: zip:
    pkgs.runCommand name
      {
        nativeBuildInputs = [ pkgs.zip ];
      }
      ''
        mkdir -p staging/${name} "$out"
        cp -R ${contents}/. staging/${name}/
        chmod -R u+w staging
        cp ${instructions} staging/${name}/README.txt
        cp ${
          pkgs.writeText "build-info.json" (
            builtins.toJSON {
              inherit revision nixpkgsRevision version;
              target = name;
            }
          )
        } staging/${name}/build-info.json
        ${
          if zip then
            ''
              find staging -exec touch -h -d @315532800 {} +
              cd staging
              zip -Xqr "$out/${name}.zip" ${name}
            ''
          else
            ''
              tar --sort=name --mtime=@1 --owner=0 --group=0 --numeric-owner \
                -czf "$out/${name}.tar.gz" -C staging ${name}
            ''
        }
        cd "$out"
        sha256sum ${name}.* > SHA256SUMS
      '';
  linuxName = "tenchu-linux-${pkgs.stdenv.hostPlatform.parsed.cpu.name}";
  hostTriplet = "${pkgs.stdenv.hostPlatform.parsed.cpu.name}-linux-gnu";
  linux =
    pkgs.runCommand linuxName
      {
        nativeBuildInputs = [
          pkgs.python3
          pkgs.patchelf
          (lib.getBin pkgs.stdenv.cc.libc)
        ];
      }
      ''
        python3 ${./bundle-linux.py} ${native}/bin/.tenchu-pc-wrapped "$out" ${dependencies.manifest} \
          ${lib.escapeShellArgs (
            map (p: "${lib.getLib p}") (
              runtimeLibraries
              ++ [
                pkgs.libpulseaudio
                pkgs.stdenv.cc.libc
              ]
            )
          )}
        mkdir -p "$out/share/X11" "$out/licenses"
        cp -RL ${pkgs.xkeyboard_config}/share/X11/xkb "$out/share/X11/"
        cp -RL ${pkgs.alsa-lib}/share/alsa "$out/share/"
        cp -R ${native}/share/tenchu-shinobi-gaisen-pc "$out/share/"
        cp -R ${native}/share/doc/tenchu-shinobi-gaisen-pc/. "$out/licenses/"
        substitute ${./tenchu-linux.sh} "$out/tenchu-pc" \
          --replace-fail '@loader@' '${builtins.baseNameOf pkgs.stdenv.cc.bintools.dynamicLinker}' \
          --replace-fail '@hostLibraries@' '/run/opengl-driver/lib:/usr/lib/${hostTriplet}:/lib/${hostTriplet}:/usr/lib64:/lib64:/usr/lib:/lib'
        chmod +x "$out/tenchu-pc"
        "$out/tenchu-pc" --help > /dev/null
      '';
  browser = pkgs.runCommand "tenchu-wasm32" { } ''
    mkdir -p "$out"
    cp -R ${web}/. "$out/"
    cp ${../tools/serve-web.py} "$out/serve-web.py"
  '';
  win = pkgs.runCommand "tenchu-windows-x86_64" { } ''
    mkdir -p "$out/licenses" "$out/assets"
    cp -R ${windows}/share/tenchu-shinobi-gaisen-pc/hd "$out/assets/hd"
    cp ${windows}/bin/tenchu-pc.exe "$out/"
    cp -R ${windows}/share/doc/tenchu-shinobi-gaisen-pc/. "$out/licenses/"
  '';
  sources = pkgs.runCommand "tenchu-dependency-sources" { } ''
    mkdir -p "$out"
    cp -R ${dependencies.sources}/. "$out/"
    cp -R ${portSource} "$out/port"
  '';
in
{
  web-archive = archive "tenchu-wasm32" browser ./README-wasm.txt false;
  dependency-sources-archive = archive "tenchu-dependency-sources" sources ./README-sources.txt false;
}
// lib.optionalAttrs pkgs.stdenv.isLinux {
  linux-bundle = linux;
  linux-archive = archive linuxName linux ./README-linux.txt false;
}
// lib.optionalAttrs (windows != null) {
  windows-archive = archive "tenchu-windows-x86_64" win ./README-windows.txt true;
}
