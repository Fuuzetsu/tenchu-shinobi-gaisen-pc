{
  description = "Tenchu: Shinobi Gaisen portable Rust PC port";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/9ff9f00c0c1a903d1d938a0dd31b8eacf8d00adc";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      runtimeLibraries =
        pkgs:
        pkgs.lib.optionals pkgs.stdenv.isLinux (
          with pkgs;
          [
            vulkan-loader
            libGL
            libxkbcommon
            wayland
            libX11
            libXcursor
            libXi
            libXrandr
          ]
        );
      source =
        lib:
        lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./crates
          ];
        };
      native =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "tenchu-shinobi-gaisen-pc";
          version = "0.1.0";
          src = source pkgs.lib;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.alsa-lib
            pkgs.udev
          ];
          postInstall = ''
            mkdir -p "$out/share/doc/tenchu-shinobi-gaisen-pc/japanese-font"
            cp crates/tenchu-assets/data/japanese-font/{OFL-1.1.txt,README.md} \
              "$out/share/doc/tenchu-shinobi-gaisen-pc/japanese-font/"
          '';
          postFixup = ''
            wrapProgram "$out/bin/tenchu-pc" \
              --set-default TENCHU_FFMPEG "${pkgs.ffmpeg}/bin/ffmpeg" \
              ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''--prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath (runtimeLibraries pkgs)}:/run/opengl-driver/lib"''}
          '';
          meta.mainProgram = "tenchu-pc";
        };
      web =
        pkgs:
        assert pkgs.wasm-bindgen-cli.version == "0.2.121";
        pkgs.rustPlatform.buildRustPackage {
          pname = "tenchu-shinobi-gaisen-web";
          version = "0.1.0";
          src = source pkgs.lib;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.wasm-bindgen-cli
            pkgs.lld
          ];
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.lld}/bin/wasm-ld";
          buildPhase = ''
            cargo build --offline --frozen --release --target wasm32-unknown-unknown -p tenchu-pc --lib
          '';
          doCheck = false;
          installPhase = ''
            mkdir -p "$out/pkg"
            wasm-bindgen --target web --out-dir "$out/pkg" target/wasm32-unknown-unknown/release/tenchu_pc.wasm
            cp crates/tenchu-pc/web/{index.html,app.js,style.css} "$out/"
            mkdir -p "$out/licenses/japanese-font"
            cp crates/tenchu-assets/data/japanese-font/{OFL-1.1.txt,README.md} "$out/licenses/japanese-font/"
          '';
        };
    in
    {
      packages = eachSystem (pkgs: {
        default = native pkgs;
        web = web pkgs;
      });
      apps = eachSystem (pkgs: {
        default = {
          type = "app";
          meta.description = "Tenchu: Shinobi Gaisen PC port";
          program = "${native pkgs}/bin/tenchu-pc";
        };
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          TENCHU_DEV_SHELL = "1";
          PLAYWRIGHT_MODULE = "${pkgs.playwright-driver}/index.mjs";
          BROWSER_EXECUTABLE = pkgs.lib.optionalString pkgs.stdenv.isLinux "${pkgs.chromium}/bin/chromium";
          PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
          TENCHU_FFMPEG = "${pkgs.ffmpeg}/bin/ffmpeg";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibraries pkgs) + ":/run/opengl-driver/lib";
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            gcc
            lld
            nixfmt
            pkg-config
            ffmpeg
            wasm-bindgen-cli
            nodejs
            python3
            playwright-driver
          ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.alsa-lib
            pkgs.udev
          ];
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.lld}/bin/wasm-ld";
        };
      });
      checks = eachSystem (pkgs: {
        native = native pkgs;
        browser = web pkgs;
        wasm = pkgs.rustPlatform.buildRustPackage {
          pname = "tenchu-portable-core";
          version = "0.1.0";
          src = source pkgs.lib;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.lld ];
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.lld}/bin/wasm-ld";
          buildPhase = ''
            cargo build --frozen --offline --release --target wasm32-unknown-unknown \
              -p tenchu-assets -p tenchu-audio -p tenchu-core --lib
          '';
          doCheck = false;
          installPhase = ''
            mkdir -p "$out/lib"
            cp target/wasm32-unknown-unknown/release/libtenchu_*.rlib "$out/lib/"
          '';
        };
        formatting =
          pkgs.runCommand "tenchu-format"
            {
              nativeBuildInputs = [
                pkgs.cargo
                pkgs.rustfmt
              ];
            }
            ''
              cd ${source pkgs.lib}
              cargo fmt --all -- --check
              touch "$out"
            '';
      });
      formatter = eachSystem (pkgs: pkgs.nixfmt);
    };
}
